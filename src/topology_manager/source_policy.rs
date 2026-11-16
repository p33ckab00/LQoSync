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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePolicy {
    pub profiles: Vec<ProfileBinding>,
    pub unprofiled_rate: RateRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePolicyIssue {
    MissingProfileMapping,
    InvalidSourceRate,
}

pub(super) struct SourceIndex<'a> {
    profiles: HashMap<(SourceKind, &'a str), &'a CurrentProfile>,
    services: HashMap<(SourceKind, &'a str), &'a CurrentService>,
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
        Ok(Self { profiles, services })
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

pub(super) struct CompiledPolicy {
    profiles: HashMap<String, Result<ObservedPolicy, SourcePolicyIssue>>,
    references: HashSet<String>,
    unprofiled_rate: RateRead,
}
impl SourcePolicy {
    pub(super) fn validate(&self, kind: SourceKind) -> anyhow::Result<()> {
        ensure!(
            kind != SourceKind::Dhcp || self.profiles.is_empty(),
            "DHCP scope cannot map account profiles"
        );
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

    pub(super) fn compile(
        &self,
        kind: SourceKind,
        index: &SourceIndex<'_>,
    ) -> anyhow::Result<CompiledPolicy> {
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
            profiles,
            references,
            unprofiled_rate: self.unprofiled_rate,
        })
    }
}
impl CompiledPolicy {
    pub(super) fn has_reference(&self, reference: &str) -> bool {
        self.references.contains(reference)
    }
    pub(super) fn observe(
        &self,
        account: &CurrentAccount,
    ) -> Result<ObservedPolicy, SourcePolicyIssue> {
        if let Some(reference) = &account.profile_ref {
            return self
                .profiles
                .get(reference)
                .cloned()
                .unwrap_or(Err(SourcePolicyIssue::MissingProfileMapping));
        }
        Ok(ObservedPolicy {
            profile: ProfileObservation::Known(None),
            plan: rate(self.unprofiled_rate, account.direct_rate_limit.as_deref())?,
        })
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
