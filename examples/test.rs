fn main()
{
    let acc: Option<u32> = Some(114514);
    let bbb = unsafe { acc.unwrap_unchecked() };

    println!("{}", bbb);
}
