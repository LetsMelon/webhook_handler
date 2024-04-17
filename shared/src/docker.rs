use serde::{Deserialize, Serialize};

// Copied from https://stackoverflow.com/a/32428199
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContainerState {
    /// A container that has been created (e.g. with `docker create`) but not started
    Created,
    /// A container that is in the process of being restarted
    Restarting,
    /// A currently running container
    Running,
    /// A container whose processes have been paused
    Paused,
    /// A container that ran and completed ("stopped" in other contexts, although a created container is technically also "stopped")
    Exited,
    /// A container that the daemon tried and failed to stop (usually due to a busy device or resource used by the container)
    Dead,
}

impl ContainerState {
    pub fn from_str(raw_status: &str) -> Result<ContainerState, &str> {
        match raw_status {
            "created" => Ok(ContainerState::Created),
            "restarting" => Ok(ContainerState::Restarting),
            "running" => Ok(ContainerState::Running),
            "paused" => Ok(ContainerState::Paused),
            "exited" => Ok(ContainerState::Exited),
            "dead" => Ok(ContainerState::Dead),
            _ => Err(raw_status),
        }
    }
}

#[derive(Serialize, Deserialize)]
// TODO maybe implement this struct myself
// what this struct can do better:
// - enums (state -> use `ContainerState`)
pub struct ContainerSummary {
    inner: bollard_stubs::models::ContainerSummary,
}

impl ContainerSummary {
    pub fn id(&self) -> &Option<String> {
        &self.inner.id
    }

    pub fn image(&self) -> &Option<String> {
        &self.inner.image
    }

    pub fn names(&self) -> &Option<Vec<String>> {
        &self.inner.names
    }

    pub fn state(&self) -> &Option<String> {
        &self.inner.state
    }

    pub fn state_enum(&self) -> Option<ContainerState> {
        self.state()
            .as_ref()
            .map(|item| ContainerState::from_str(&item).ok())
            .flatten()
    }
}
