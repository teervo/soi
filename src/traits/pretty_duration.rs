pub trait PrettyDuration {
    fn pretty(&self) -> String;
}

/// Formats Duration for human consumption, "(mm:ss)".
impl PrettyDuration for std::time::Duration {
    fn pretty(&self) -> String {
        let minutes = self.as_secs() / 60;
        let seconds = self.as_secs() % 60;
        format!("{}:{:02}", minutes, seconds)
    }
}
