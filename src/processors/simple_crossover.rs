use crossbeam::channel;
use crate::processors::Aggregator;
use crate::messages::{Msg, PriceUpdated};

pub struct SimpleCrossover {}

impl<'a> Aggregator<'a> for SimpleCrossover {
    fn aggregate(&mut self, msg: &Msg<'a>) -> Vec<Msg<'a>> {
        match msg {
            Msg::LivePriceUpdated(e) => vec![],
            _ => return vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

  /*   #[test]
    fn strategy_should_exit_if_shutdown_received() {
        let (strategy, in_s, _) = new_strategy(false);
        in_s.send(Msg::Shutdown).unwrap();
        strategy.start();
        assert!(true);
    } */
}