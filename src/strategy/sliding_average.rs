use crate::messaging::message::{Msg, MsgData, PriceUpdated};
use crate::messaging::processor::Actor;
use anyhow::Result;
use async_trait::async_trait;

pub type Timestamp = u128;
pub type Price = f64;

#[derive(Debug, PartialEq, Clone, Default)]
struct TimePricePoint {
    datetime: Timestamp,
    price: Price,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SlidingAverage {
    pub window_millis: u128,
    events: Vec<TimePricePoint>,
    active: bool,
}

impl SlidingAverage {
    pub fn new(window_millis: u128) -> Self {
        SlidingAverage {
            window_millis,
            events: vec![],
            active: false
        }
    }
}

#[async_trait]
impl Actor for SlidingAverage {
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>> {
        let res = match &msg.data {
            MsgData::LivePriceUpdated(e) => {
                self.events.push(TimePricePoint {
                    datetime: e.datetime,
                    price: e.price,
                });
                let before_len = self.events.len();
                self.events
                    .retain(|i| i.datetime as i128 >= e.datetime as i128 - self.window_millis as i128);
                let sum: f64 = self.events.iter().map(|e| e.price).sum();
                let avg = PriceUpdated {
                    pair_id: e.pair_id,
                    datetime: e.datetime,
                    price: sum / self.events.len() as f64,
                    ..Default::default()
                };
                if before_len > self.events.len() {
                    self.active = true;
                }
                if self.active {
                    vec![MsgData::AveragePriceUpdated(avg)]
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

    const SECOND: u128 = 1_000;

    #[async_std::test]
    async fn actor_should_emit_average_price_update() {
        let mut actor = SlidingAverage::new(SECOND);
        let e1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let e2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND + 1,
            price: 2.0,
            ..Default::default()
        }));
        actor.act(&e1).await.unwrap();
        let actual_e = actor.act(&e2).await.unwrap();
        let expected_e = vec![MsgData::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND + 1,
            price: 2.0,
            ..Default::default()
        })];
        assert_eq!(expected_e, actual_e)
    }

    #[async_std::test]
    async fn actor_should_calculate_prices_from_given_sliding_window() {
        let mut actor = SlidingAverage::new(SECOND);
        let e1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let e2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        }));
        let e3 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 3.0,
            ..Default::default()
        }));
        actor.act(&e1).await.unwrap();
        actor.act(&e2).await.unwrap();
        let actual = actor.act(&e3).await.unwrap();

        let expected_e2 = vec![MsgData::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 2.5,
            ..Default::default()
        })];
        assert_eq!(expected_e2, actual);
    }

    #[async_std::test]
    async fn actor_should_not_emit_average_price_update_if_window_not_full() {
        let mut actor = SlidingAverage::new(SECOND * 2);
        let e1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let e2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        }));
        actor.act(&e1).await.unwrap();
        let actual_e = actor.act(&e2).await.unwrap();
        let expected_e: Vec<MsgData> = vec![];
        assert_eq!(expected_e, actual_e)
    }
}
