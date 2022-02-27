use crate::messaging::message::{Msg, MsgData, PriceUpdated};
use crate::messaging::processor::Actor;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SlidingAverage {
    pub window_millis: u128,
    latest_average: Option<f64>,
    counted_price_points: u16,
    min_price_points: u16,
}

impl SlidingAverage {
    pub fn new(interval_millis: u128, window_millis: u128) -> Self {
        SlidingAverage {
            window_millis,
            latest_average: None,
            counted_price_points: 0,
            min_price_points: (window_millis / interval_millis) as u16,
        }
    }
}

#[async_trait]
impl Actor for SlidingAverage {
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>> {
        let res = match &msg.data {
            MsgData::LivePriceUpdated(e) => {
                let latest_average = self.latest_average.unwrap_or(e.price);
                let current_average = (e.price - latest_average)
                    * (2.0 / (self.min_price_points + 1) as f64)
                    + latest_average;
                self.latest_average = Some(current_average);
                if self.counted_price_points >= self.min_price_points {
                    vec![MsgData::AveragePriceUpdated(PriceUpdated {
                        pair_id: e.pair_id,
                        datetime: e.datetime,
                        price: current_average,
                        ..Default::default()
                    })]
                } else {
                    self.counted_price_points += 1;
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
        let mut actor = SlidingAverage::new(SECOND, SECOND);
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
        let mut actor = SlidingAverage::new(SECOND, SECOND * 2);
        let e1 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: 0,
            price: 1.0,
            ..Default::default()
        }));
        let e2 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 2.2,
            ..Default::default()
        }));
        let e3 = Msg::with_data(MsgData::LivePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 4.5,
            ..Default::default()
        }));
        actor.act(&e1).await.unwrap();
        actor.act(&e2).await.unwrap();
        let actual = actor.act(&e3).await.unwrap();

        let expected_e2 = vec![MsgData::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 3.6,
            ..Default::default()
        })];
        assert_eq!(expected_e2, actual);
    }

    #[async_std::test]
    async fn actor_should_not_emit_average_price_update_if_window_not_full() {
        let mut actor = SlidingAverage::new(SECOND, SECOND * 2);
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
