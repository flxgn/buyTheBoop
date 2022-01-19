use uuid::Uuid;

pub type Timestamp = i64;
pub type Price = f64;
pub type PairId<'a> = &'a str;
pub type EventId = Uuid;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct PriceUpdated<'a> {
    pub id: EventId,
    pub pair_id: PairId<'a>,
    pub datetime: Timestamp,
    pub price: Price,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(PartialEq, Debug, Clone)]
pub struct MarketOrder {
    pub bid_currency: String,
    pub ask_currency: String,
    pub order_type: OrderType,
    pub amount: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Msg<'a> {
    LivePriceUpdated(PriceUpdated<'a>),
    AveragePriceUpdated(PriceUpdated<'a>),
    OrderExecuted(MarketOrder),
    Buy,
    Sell,
    Shutdown,
}
