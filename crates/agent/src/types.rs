use crate::workers::BuildJob;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct AgentState {
    pub build_queue: Sender<BuildJob>,
}
