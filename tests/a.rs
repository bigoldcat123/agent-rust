use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Intent {
    Ticket,
    Arrange,
    Other,
}

#[test]
fn feature() {
    let a = Intent::Arrange;
    let a = serde_json::to_string(&a).unwrap();
    println!("{} ?", a);
    let a: Intent = serde_json::from_str(a.as_str()).unwrap();
    println!("{:?}", a);
}
