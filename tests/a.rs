use serde::{Deserialize, Serialize};
use serde_json::ser;

#[derive(Serialize,Deserialize,Debug)]
pub enum Intent {
    Ticket,
    Arrange,
    Other
}

#[test]
fn feature() {
    let a = Intent::Arrange;
    let a = serde_json::to_string(&a).unwrap();
    println!("{}",a);
    let a:Intent = serde_json::from_str(a.as_str()).unwrap();
    println!("{:?}",a);
}
