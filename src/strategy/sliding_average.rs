use crate::messaging::message::{Msg, PriceUpdated};
use crate::messaging::processor::Actor;
use anyhow::Result;
use async_trait::async_trait;

pub type Timestamp = i64;
pub type Price = f64;

#[derive(Debug, PartialEq, Clone, Default)]
struct TimePricePoint {
    datetime: Timestamp,
    price: Price,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SlidingAverage {
    pub window_millis: i64,
    events: Vec<TimePricePoint>,
}

impl SlidingAverage {
    pub fn new(window_millis: i64) -> Self {
        SlidingAverage {
            window_millis,
            events: vec![],
        }
    }
}

#[async_trait]
impl Actor for SlidingAverage {
    async fn act(&mut self, msg: &Msg) -> Result<Vec<Msg>> {
        let res = match msg {
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
                    vec![Msg::AveragePriceUpdated(avg)]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SECOND: i64 = 1_000;

    #[async_std::test]
    async fn actor_should_emit_average_price_update() {
        let mut actor = SlidingAverage::new(SECOND);
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
        actor.act(&e1).await.unwrap();
        let actual_e = actor.act(&e2).await.unwrap();
        let expected_e = vec![Msg::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.5,
            ..Default::default()
        })];
        assert_eq!(expected_e, actual_e)
    }

    #[async_std::test]
    async fn actor_should_calculate_prices_from_given_sliding_window() {
        let mut actor = SlidingAverage::new(SECOND);
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
        actor.act(&e1).await.unwrap();
        let actual_e1 = actor.act(&e2).await.unwrap();
        let actual_e2 = actor.act(&e3).await.unwrap();

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
