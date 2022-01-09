use crossbeam::channel;
use uuid::Uuid;

pub type Timestamp = i64;
pub type Price = f64;
pub type PairId<'a> = &'a str;
pub type EventId = Uuid;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct PriceUpdated<'a> {
    id: EventId,
    pair_id: PairId<'a>,
    datetime: Timestamp,
    price: Price,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Event<'a> {
    LivePriceUpdated(PriceUpdated<'a>),
    AveragePriceUpdated(PriceUpdated<'a>),
    Shutdown,
}

pub struct Processor<'a> {
    pub input: channel::Receiver<Event<'a>>,
    pub output: channel::Sender<Event<'a>>,
    pub is_filter: bool,
    pub window_millis: i64,

    events: Vec<TimePricePoint>,
}

struct TimePricePoint {
    datetime: Timestamp,
    price: Price,
}

impl<'a> Processor<'a> {
    pub fn start(mut self) {
        loop {
            let e = self.input.recv().expect("open channel");
            match &e {
                Event::Shutdown => break,
                Event::LivePriceUpdated(e) => {
                    self.events.push(TimePricePoint {
                        datetime: e.datetime,
                        price: e.price,
                    });
                    self.events.retain(|i| i.datetime >= e.datetime - self.window_millis as i64 );
                    let sum: f64 = self.events.iter().map(|e| e.price).sum();
                    let avg = PriceUpdated {
                        pair_id: e.pair_id,
                        datetime: e.datetime,
                        price: sum / self.events.len() as f64,
                        ..Default::default()
                    };
                    if self.events.len() > 1 {
                        self.output
                            .send(Event::AveragePriceUpdated(avg))
                            .expect("open channel");
                    }
                }
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

    const SECOND: i64 = 1_000;

    fn new_processor<'a>(
        window_millis: i64,
        is_filter: bool,
    ) -> (
        Processor<'a>,
        channel::Sender<Event<'a>>,
        channel::Receiver<Event<'a>>,
    ) {
        let (in_s, in_r) = unbounded();
        let (out_s, out_r) = unbounded();
        (
            Processor {
                input: in_r,
                output: out_s,
                is_filter,
                window_millis,
                events: vec![],
            },
            in_s,
            out_r,
        )
    }

    #[test]
    fn processor_should_exit_if_shutdown_received() {
        let (processor, in_s, _) = new_processor(0, false);
        in_s.send(Event::Shutdown).unwrap();
        processor.start();
        assert!(true);
    }

    #[test]
    fn processor_should_output_events_if_not_filtered() {
        let (processor, in_s, out_r) = new_processor(0, false);
        let expected_e = Event::LivePriceUpdated(PriceUpdated {
            ..Default::default()
        });
        in_s.send(expected_e.clone()).unwrap();
        in_s.send(Event::Shutdown).unwrap();
        processor.start();
        let actual_e = out_r.recv().unwrap();
        assert_eq!(expected_e, actual_e);
    }

    #[test]
    fn processor_should_emit_average_price_update() {
        let (processor, in_s, out_r) = new_processor(SECOND, true);
        let e1 = Event::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let e2 = Event::LivePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        });
        in_s.send(e1).unwrap();
        in_s.send(e2).unwrap();
        in_s.send(Event::Shutdown).unwrap();
        processor.start();

        let actual_e = out_r.recv().unwrap();
        let expected_e = Event::AveragePriceUpdated(PriceUpdated {
            pair_id: "pair_id",
            datetime: SECOND,
            price: 1.5,
            ..Default::default()
        });
        assert_eq!(expected_e, actual_e)
    }

    #[test]
    fn processor_should_calculate_prices_from_given_sliding_window() {
        let (processor, in_s, out_r) = new_processor(SECOND, true);
        let e1 = Event::LivePriceUpdated(PriceUpdated {
            datetime: 0,
            price: 1.0,
            ..Default::default()
        });
        let e2 = Event::LivePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 2.0,
            ..Default::default()
        });
        let e3 = Event::LivePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 3.0,
            ..Default::default()
        });
        in_s.send(e1).unwrap();
        in_s.send(e2).unwrap();
        in_s.send(e3).unwrap();
        in_s.send(Event::Shutdown).unwrap();
        processor.start();

        let actual_e1 = out_r.recv().unwrap();
        let expected_e1 = Event::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND,
            price: 1.5,
            ..Default::default()
        });
        assert_eq!(expected_e1, actual_e1);

        let actual_e2 = out_r.recv().unwrap();
        let expected_e2 = Event::AveragePriceUpdated(PriceUpdated {
            datetime: SECOND * 2,
            price: 2.5,
            ..Default::default()
        });
        assert_eq!(expected_e2, actual_e2);
    }
}
