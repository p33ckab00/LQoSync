//! Explicit current profile selectors and source rate read modes.

use super::model::SourceKind;
use super::normalize::{CurrentAccount, CurrentProfile, CurrentRouter, CurrentService};
use super::observation::{ObservedPolicy, ProfileObservation};
use super::source_rate::{RateFormat, parse_rate_limit};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateRead {
    #[default]
    Disabled,
    RateLimit(RateFormat),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileBinding {
    pub source_ref: String,
    pub current_name: String,
    pub rate: RateRead,
}

/// Explicit migration intent. The legacy position locates a rate entry, never a subscriber.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpOverrideBinding {
    pub legacy_client_index: usize,
    pub account_ref: String,
    pub rate: super::rates::RateBps,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePolicy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dhcp_overrides: Vec<DhcpOverrideBinding>,
    pub profiles: Vec<ProfileBinding>,
    pub unprofiled_rate: RateRead,
    #[serde(default)]
    pub legacy_priority: Option<super::legacy_rate::LegacyPriority>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePolicyIssue {
    MissingProfileMapping,
    InvalidSourceRate,
}

pub(super) struct SourceIndex<'a> {
    profiles: HashMap<(SourceKind, &'a str), &'a CurrentProfile>,
    services: HashMap<(SourceKind, &'a str), &'a CurrentService>,
    profiles_by_ref: HashMap<(SourceKind, &'a str), &'a CurrentProfile>,
    services_by_ref: HashMap<(SourceKind, &'a str), &'a CurrentService>,
}
impl<'a> SourceIndex<'a> {
    pub fn new(current: &'a CurrentRouter) -> anyhow::Result<Self> {
        let profiles = current
            .profiles()?
            .iter()
            .map(|p| ((p.kind, p.name.as_str()), p))
            .collect();
        let services = current
            .services()?
            .iter()
            .map(|service| ((service.kind, service.name.as_str()), service))
            .collect();
        Ok(Self {
            profiles,
            services,
            profiles_by_ref: current
                .profiles()?
                .iter()
                .map(|p| ((p.kind, p.row_ref.as_str()), p))
                .collect(),
            services_by_ref: current
                .services()?
                .iter()
                .map(|s| ((s.kind, s.row_ref.as_str()), s))
                .collect(),
        })
    }
    pub fn service_ref(&self, kind: SourceKind, name: &str) -> anyhow::Result<String> {
        if kind == SourceKind::Ppp {
            ensure!(
                matches!(
                    name,
                    "async" | "isdn" | "l2tp" | "pppoe" | "pptp" | "ovpn" | "sstp"
                ),
                "unknown PPP service selector"
            );
            return Ok(name.to_owned());
        }
        let service = self
            .services
            .get(&(kind, name))
            .context("selected current service is unavailable")?;
        Ok(service.row_ref.clone())
    }
}

pub(super) struct CompiledPolicy<'a, 'b> {
    index: &'b SourceIndex<'a>,
    priority: Option<super::legacy_rate::CompiledPriority>,
    profiles: HashMap<String, Result<ObservedPolicy, SourcePolicyIssue>>,
    references: HashSet<String>,
    unprofiled_rate: RateRead,
    dhcp_overrides: HashMap<String, super::rates::RateBps>,
}
impl SourcePolicy {
    pub(super) fn validate(&self, kind: SourceKind) -> anyhow::Result<()> {
        ensure!(
            kind != SourceKind::Dhcp || self.profiles.is_empty(),
            "DHCP scope cannot map account profiles"
        );
        ensure!(
            kind == SourceKind::Dhcp || self.dhcp_overrides.is_empty(),
            "subscriber overrides require a DHCP scope"
        );
        let mut positions = HashSet::new();
        let mut accounts = HashSet::new();
        for binding in &self.dhcp_overrides {
            ensure!(
                !binding.account_ref.trim().is_empty(),
                "empty override account reference"
            );
            binding.rate.validate()?;
            ensure!(
                positions.insert(binding.legacy_client_index)
                    && accounts.insert(binding.account_ref.as_str()),
                "duplicate DHCP override binding"
            );
        }
        if let Some(priority) = &self.legacy_priority {
            priority.compile(kind)?;
            ensure!(
                self.unprofiled_rate == RateRead::Disabled
                    && self.profiles.iter().all(|b| b.rate == RateRead::Disabled),
                "legacy priority conflicts with another source rate authority"
            );
        }
        let mut references = HashSet::new();
        let mut names = HashSet::new();
        for binding in &self.profiles {
            ensure!(
                !binding.source_ref.trim().is_empty() && !binding.current_name.trim().is_empty(),
                "source profile binding must not be blank"
            );
            ensure!(
                references.insert(binding.source_ref.as_str()),
                "duplicate stable source profile reference"
            );
            ensure!(
                names.insert(binding.current_name.as_str()),
                "duplicate current profile selector"
            );
        }
        Ok(())
    }

    pub(super) fn compile<'a, 'b>(
        &self,
        kind: SourceKind,
        index: &'b SourceIndex<'a>,
    ) -> anyhow::Result<CompiledPolicy<'a, 'b>> {
        self.validate(kind)?;
        let mut profiles = HashMap::with_capacity(self.profiles.len());
        let references = self.profiles.iter().map(|b| b.source_ref.clone()).collect();
        for binding in &self.profiles {
            if let Some(profile) = index.profiles.get(&(kind, binding.current_name.as_str())) {
                let observed =
                    rate(binding.rate, profile.rate_limit.as_deref()).map(|plan| ObservedPolicy {
                        profile: ProfileObservation::Known(Some(binding.source_ref.clone())),
                        plan,
                    });
                ensure!(
                    profiles.insert(profile.row_ref.clone(), observed).is_none(),
                    "ambiguous current source profile locator"
                );
            }
        }
        Ok(CompiledPolicy {
            index,
            priority: self
                .legacy_priority
                .as_ref()
                .map(|p| p.compile(kind))
                .transpose()?,
            profiles,
            references,
            unprofiled_rate: self.unprofiled_rate,
            dhcp_overrides: self
                .dhcp_overrides
                .iter()
                .map(|b| (b.account_ref.clone(), b.rate))
                .collect(),
        })
    }
}
impl CompiledPolicy<'_, '_> {
    pub(super) fn has_reference(&self, reference: &str) -> bool {
        self.references.contains(reference)
    }
    pub(super) fn observe_identified(
        &self,
        account: &CurrentAccount,
        account_ref: &str,
    ) -> Result<ObservedPolicy, SourcePolicyIssue> {
        if let Some(rate) = self.dhcp_overrides.get(account_ref) {
            return Ok(ObservedPolicy {
                profile: ProfileObservation::Known(None),
                plan: Some(*rate),
            });
        }
        self.observe(account)
    }
    pub(super) fn observe(
        &self,
        account: &CurrentAccount,
    ) -> Result<ObservedPolicy, SourcePolicyIssue> {
        let mut observed = if let Some(reference) = &account.profile_ref {
            self.profiles
                .get(reference)
                .cloned()
                .unwrap_or(Err(SourcePolicyIssue::MissingProfileMapping))?
        } else {
            ObservedPolicy {
                profile: ProfileObservation::Known(None),
                plan: rate(self.unprofiled_rate, account.direct_rate_limit.as_deref())?,
            }
        };
        if let Some(priority) = &self.priority {
            let services: Vec<_> = account
                .service_refs
                .iter()
                .filter_map(|id| {
                    self.index
                        .services_by_ref
                        .get(&(account.kind, id.as_str()))
                        .copied()
                })
                .collect();
            observed.plan = priority
                .read(
                    account,
                    account.profile_ref.as_deref().and_then(|id| {
                        self.index.profiles_by_ref.get(&(account.kind, id)).copied()
                    }),
                    &services,
                )
                .map_err(|_| SourcePolicyIssue::InvalidSourceRate)?;
        }
        Ok(observed)
    }
}

fn rate(
    read: RateRead,
    text: Option<&str>,
) -> Result<Option<super::rates::RateBps>, SourcePolicyIssue> {
    match read {
        RateRead::Disabled => Ok(None),
        RateRead::RateLimit(format) => {
            parse_rate_limit(text, format).map_err(|_| SourcePolicyIssue::InvalidSourceRate)
        }
    }
}

#[cfg(test)]
#[path = "source_policy_tests.rs"]
mod tests;
