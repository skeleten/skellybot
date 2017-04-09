use discord::model::*;
use ::error::*;
use ::context::*;
use std::collections::HashMap;

pub trait HandlerFunc where Self : Fn(&Message, &mut Context) -> Result<bool> { }
impl<T: Fn(&Message, &mut Context) -> Result<bool>> HandlerFunc for T { }

pub struct MessageHandlerStore {
    handlers: HashMap<String, Vec<Box<HandlerFunc<Output = Result<bool>>>>>,
}

impl MessageHandlerStore {
    pub fn new() -> Self {
        MessageHandlerStore {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<F: 'static + HandlerFunc>(&mut self,
                                                      keyword: &str,
                                                      handler: F) {
        let contained = self.handlers.contains_key(keyword);
        if contained {
            if let Some(ref mut v) = self.handlers.get_mut(keyword) {
                v.push(Box::new(handler))
            } else {
                unreachable!();
            }
        } else {
            self.handlers.insert(keyword.into(), vec![ Box::new(handler) ]);
        }
    }

    pub fn call_handler(&self, msg: Message, ctx: &mut Context) -> Result<bool> {
        let parts = msg.content.split_whitespace().collect::<Vec<_>>();
        let keyword = parts[0];
        if let Some(ref handlers) = self.handlers.get(keyword) {
            for h in handlers.iter() {
                match h(&msg, ctx) {
                    Ok(true)  => break,
                    Ok(false) => continue,
                    Err(e)    => {
                        let mut m = String::new();
                        m += &format!("Error: {}", e);
                        for e in e.iter().skip(1) {
                            m +=  &format!("\ncaused by: {}", e);
                        }
                        if let Some(bt) = e.backtrace() {
                            m += &format!("\n\nbacktrace: ```\n{:?}", bt);
                        }

                        ctx.client.send_message(&msg.channel_id, &m, "", false)?;
                    }
                }
            }
        }

        Ok(true)
    }

    pub fn get_handler_count(&self) -> usize {
        self.handlers.values().map(|v| v.len()).sum()
    }
}
