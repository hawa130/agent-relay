use crate::models::{Profile, ProfileProbeIdentity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLinkResult {
    pub profile: Profile,
    pub probe_identity: ProfileProbeIdentity,
    pub activated: bool,
}
