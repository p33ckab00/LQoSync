//! Explicit conversion of legacy speed priorities. Dynamic source text stays in memory.
use super::model::SourceKind;
use super::normalize::{AuthorityEvidence, CurrentAccount, CurrentProfile, CurrentService};
use super::rates::RateBps;
use super::source_rate::RateFormat;
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyRateSource {
    AccountComment,
    ActiveComment,
    ProfileComment,
    ProfileRateLimit,
    ProfileName,
    ServerComment,
    Settings,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyPriority {
    pub kind: SourceKind,
    pub sources: Vec<LegacyRateSource>,
    pub format: RateFormat,
    pub profile_name_regex: String,
    /// PPP always has a configured fallback. DHCP has one only when Settings is selected.
    pub fallback: Option<RateBps>,
    pub settings: RateBps,
}
impl LegacyPriority {
    pub fn ppp(speed: &crate::config::Speed, default: &str) -> anyhow::Result<Self> {
        use crate::config::PppSource as P;
        let settings = RateBps::try_from(
            crate::speed::parse_default_rate(default).context("invalid explicit PPP default")?,
        )?;
        let value = Self {
            kind: SourceKind::Ppp,
            sources: speed
                .pppoe_sources
                .iter()
                .map(|s| match s {
                    P::SecretComment => LegacyRateSource::AccountComment,
                    P::ActiveComment => LegacyRateSource::ActiveComment,
                    P::ProfileComment => LegacyRateSource::ProfileComment,
                    P::ProfileRateLimit => LegacyRateSource::ProfileRateLimit,
                    P::ProfileName => LegacyRateSource::ProfileName,
                    P::Default => LegacyRateSource::Settings,
                })
                .collect(),
            format: RateFormat {
                order: speed.rate_limit_order,
                unitless: speed.unitless_unit,
            },
            profile_name_regex: speed.profile_name_regex.clone(),
            fallback: Some(settings),
            settings,
        };
        value.compile(SourceKind::Ppp)?;
        Ok(value)
    }
    pub fn dhcp(
        speed: &crate::config::Speed,
        defaults: &crate::config::BandwidthDefaults,
        server: &crate::config::DhcpServer,
    ) -> anyhow::Result<Self> {
        use crate::config::DhcpSource as D;
        let value = Self {
            kind: SourceKind::Dhcp,
            sources: server
                .sources(speed)
                .iter()
                .map(|s| match s {
                    D::LeaseComment => LegacyRateSource::AccountComment,
                    D::ServerComment => LegacyRateSource::ServerComment,
                    D::Settings => LegacyRateSource::Settings,
                })
                .collect(),
            format: RateFormat {
                order: speed.rate_limit_order,
                unitless: speed.unitless_unit,
            },
            profile_name_regex: speed.profile_name_regex.clone(),
            fallback: None,
            settings: RateBps::try_from(server.settings_rate(defaults))?,
        };
        ensure!(
            server.mode != crate::config::DhcpMode::PerSite
                || !value.sources.contains(&LegacyRateSource::AccountComment),
            "per-site policy cannot use lease comments"
        );
        value.compile(SourceKind::Dhcp)?;
        Ok(value)
    }
    pub fn hotspot(
        speed: &crate::config::Speed,
        defaults: &crate::config::BandwidthDefaults,
        hotspot: &crate::config::HotspotCfg,
    ) -> anyhow::Result<Self> {
        let settings = RateBps::try_from(hotspot.per_client(defaults))?;
        Ok(Self {
            kind: SourceKind::Hotspot,
            sources: vec![LegacyRateSource::Settings],
            format: RateFormat {
                order: speed.rate_limit_order,
                unitless: speed.unitless_unit,
            },
            profile_name_regex: speed.profile_name_regex.clone(),
            fallback: None,
            settings,
        })
    }
    pub(super) fn compile(&self, kind: SourceKind) -> anyhow::Result<CompiledPriority> {
        ensure!(self.kind == kind, "legacy priority protocol mismatch");
        self.settings.validate()?;
        if let Some(rate) = self.fallback {
            rate.validate()?;
        }
        ensure!(
            self.sources.iter().all(|s| match kind {
                SourceKind::Ppp => !matches!(s, LegacyRateSource::ServerComment),
                SourceKind::Dhcp => matches!(
                    s,
                    LegacyRateSource::AccountComment
                        | LegacyRateSource::ServerComment
                        | LegacyRateSource::Settings
                ),
                SourceKind::Hotspot => matches!(s, LegacyRateSource::Settings),
            }),
            "unsupported legacy priority source"
        );
        let parser = crate::speed::SpeedParser::new(&crate::config::Speed {
            rate_limit_order: self.format.order,
            unitless_unit: self.format.unitless,
            profile_name_regex: self.profile_name_regex.clone(),
            ..Default::default()
        })?;
        Ok(CompiledPriority {
            rule: self.clone(),
            parser,
        })
    }
}
pub(super) struct CompiledPriority {
    rule: LegacyPriority,
    parser: crate::speed::SpeedParser,
}
impl CompiledPriority {
    pub fn read(
        &self,
        account: &CurrentAccount,
        profile: Option<&CurrentProfile>,
        services: &[&CurrentService],
    ) -> anyhow::Result<Option<RateBps>> {
        ensure!(
            account.kind == self.rule.kind,
            "current account protocol mismatch"
        );
        let comment = match &account.authority {
            AuthorityEvidence::LocalAccount { comment, .. }
            | AuthorityEvidence::EndpointOnly { comment, .. } => comment.as_deref(),
        };
        let read = |active_comment: Option<&str>| -> anyhow::Result<Option<RateBps>> {
            for source in &self.rule.sources {
                if *source == LegacyRateSource::Settings {
                    return Ok(Some(self.rule.settings));
                }
                let value = match source {
                    LegacyRateSource::AccountComment => {
                        self.parser.comment(comment.unwrap_or_default())
                    }
                    LegacyRateSource::ActiveComment => {
                        self.parser.comment(active_comment.unwrap_or_default())
                    }
                    LegacyRateSource::ProfileComment => profile.and_then(|p| {
                        self.parser
                            .comment(p.comment.as_deref().unwrap_or_default())
                    }),
                    LegacyRateSource::ProfileRateLimit => profile.and_then(|p| {
                        self.parser
                            .rate_limit(p.rate_limit.as_deref().unwrap_or_default())
                    }),
                    LegacyRateSource::ProfileName => {
                        profile.and_then(|p| self.parser.profile_name(&p.name))
                    }
                    LegacyRateSource::ServerComment => {
                        ensure!(
                            services.len() == 1,
                            "DHCP priority requires one current service"
                        );
                        self.parser
                            .comment(services[0].comment.as_deref().unwrap_or_default())
                    }
                    LegacyRateSource::Settings => unreachable!(),
                };
                if let Some(value) = value {
                    return Ok(Some(RateBps::try_from(value)?));
                }
            }
            Ok(self.rule.fallback)
        };
        let first = read(account.active_comments.first().and_then(|s| s.as_deref()))?;
        if self.rule.sources.contains(&LegacyRateSource::ActiveComment) {
            for comment in account.active_comments.iter().skip(1) {
                ensure!(
                    read(comment.as_deref())? == first,
                    "current sessions have conflicting source speeds"
                );
            }
        }
        Ok(first)
    }
}

#[cfg(test)]
#[path = "legacy_rate_tests.rs"]
mod tests;
