use crate::models::{FailureEvent, Profile, RelayError};
use chrono::Utc;

pub fn select_next_profile(
    profiles: &[Profile],
    active_profile_id: Option<&str>,
    failure_events: &[FailureEvent],
) -> Result<Profile, RelayError> {
    let eligible = profiles
        .iter()
        .filter(|profile| profile.enabled)
        .filter(|profile| !is_in_cooldown(profile, failure_events))
        .collect::<Vec<_>>();

    if eligible.is_empty() {
        return Err(RelayError::NotFound("no eligible profile available".into()));
    }

    if let Some(active_profile_id) = active_profile_id {
        if eligible.len() == 1 && eligible[0].id == active_profile_id {
            return Err(RelayError::NotFound("no next profile available".into()));
        }

        if let Some(index) = eligible
            .iter()
            .position(|profile| profile.id == active_profile_id)
        {
            return Ok(eligible[(index + 1) % eligible.len()].clone());
        }
    }

    Ok(eligible[0].clone())
}

fn is_in_cooldown(profile: &Profile, events: &[FailureEvent]) -> bool {
    let now = Utc::now();
    events.iter().any(|event| {
        event.profile_id.as_deref() == Some(profile.id.as_str())
            && event.cooldown_until.is_some_and(|until| until > now)
    })
}
