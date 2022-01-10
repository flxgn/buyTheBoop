use crossbeam::channel;
use crate::messages::{Msg, PriceUpdated};

pub struct Strategy<'a> {
    pub input: channel::Receiver<Msg<'a>>,
    pub output: channel::Sender<Msg<'a>>,
    pub is_filter: bool,
}

impl<'a> Strategy<'a> {
    pub fn start(mut self) {
        loop {
            let e = self.input.recv().expect("open channel");
            match &e {
                Msg::Shutdown => (),
                Msg::LivePriceUpdated(_) => (),
                _ => (),
            }
            if !self.is_filter {
                self.output.send(e).expect("open channel")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::unbounded;
    use pretty_assertions::assert_eq;

    fn new_strategy<'a>(is_filter: bool) -> (
        Strategy<'a>,
        channel::Sender<Msg<'a>>,
        channel::Receiver<Msg<'a>>,
    ) {
        let (in_s, in_r) = unbounded();
        let (out_s, out_r) = unbounded();
        (
            Strategy {
                input: in_r,
                output: out_s,
                is_filter
            },
            in_s,
            out_r,
        )
    }

    #[test]
    fn strategy_should_exit_if_shutdown_received() {
        let (strategy, in_s, _) = new_strategy(false);
        in_s.send(Msg::Shutdown).unwrap();
        strategy.start();
        assert!(true);
    }
}