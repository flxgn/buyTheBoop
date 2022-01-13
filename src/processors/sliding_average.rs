use crate::messages::{Msg, PriceUpdated};
use crate::processors::Aggregator;

pub type Timestamp = i64;
pub type Price = f64;

#[derive(Debug, PartialEq, Clone, Default)]
struct TimePricePoint {
    datetime: Timestamp,
    price: Price,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SlidingAverageAggregator {
    pub window_millis: i64,
    events: Vec<TimePricePoint>,
}

impl SlidingAverageAggregator {
    pub fn new(window_millis: i64) -> Self {
        SlidingAverageAggregator {
            window_millis,
            events: vec![],
        }
    }
}

impl<'a> Aggregator<'a> for SlidingAverageAggregator {
    fn aggregate(&mut self, msg: &Msg<'a>) -> Vec<Msg<'a>> {
        match msg {
            Msg::LivePriceUpdated(e) => {
                self.events.push(TimePricePoint {
                    datetime: e.datetime,
                    price: e.price,
                });
                self.events
                    .retain(|i| i.datetime >= e.datetime - self.window_millis as i64);
                let sum: f64 = self.events.iter().map(|e| e.price).sum();
                let avg = PriceUpdated {
                    pair_id: e.pair_id,
                    datetime: e.datetime,
                    price: sum / self.events.len() as f64,
                    ..Default::default()
                };
                if self.events.len() > 1 {
                    return vec![Msg::AveragePriceUpdated(avg)];
                } else {
                    return vec![];
                }
            }
            _ => return vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SECOND: i64 = 1_000;

    #[test]
    fn aggr_should_emit_average_price_update() {
        let mut aggregator = SlidingAverageAggregator::new(SECOND);
        let e1 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let e2 = Msg::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        });
        aggregator.aggregate(&e1);
        let actual_e = aggregator.aggregate(&e2);
        let expected_e = vec![Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.5,
            ..Default::default()
        })];
        assert_eq!(expected_e, actual_e)
    }

    #[test]
    fn aggr_should_calculate_prices_from_given_sliding_window() {
        let mut aggregator = SlidingAverageAggregator::new(SECOND);
        let e1 = Msg::LivePriceUpdated(PriceUpdated {
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let e2 = Msg::LivePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        });
        let e3 = Msg::LivePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 3.0,
            ..Default::default()
        });
        aggregator.aggregate(&e1);
        let actual_e1 = aggregator.aggregate(&e2);
        let actual_e2 = aggregator.aggregate(&e3);

        let expected_e1 = vec![Msg::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 1.5,
            ..Default::default()
        })];
        assert_eq!(expected_e1, actual_e1);

        let expected_e2 = vec![Msg::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 2.5,
            ..Default::default()
        })];
        assert_eq!(expected_e2, actual_e2);
    }
}
