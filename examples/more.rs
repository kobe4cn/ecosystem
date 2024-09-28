use anyhow::Result;
use derive_more::{derive::Into, Add, Display, From};

#[derive(Debug, Display, From, Add, PartialEq, Into)]
struct MyInt(i32);

#[derive(PartialEq, From)]
struct Point2D {
    x: i32,
    y: i32,
}
#[derive(PartialEq, From, Add, Display)]
enum MyNum {
    #[display("int: {_0}")]
    Int(i32),
    Uint(u32),
    #[display("nothing")]
    Nothing,
}

fn main() -> Result<()> {
    let my_int: MyInt = 10.into();
    let v: MyInt = my_int + 20.into();
    println!("{}", v);

    let v1: i32 = v.into();
    println!("{}", v1);

    let n1 = MyNum::Uint(20);
    println!("{}", n1);

    let no: MyNum = MyNum::Nothing;
    println!("{}", no);

    let n2 = MyNum::Int(30);
    println!("{}", n2);

    Ok(())
}
