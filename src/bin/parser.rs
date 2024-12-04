use mml_parser::{self, MmlAst};

fn main() -> anyhow::Result<()> {
    let to_parse = r#"
        cdef1,12,1aq100n64,,30,10
    "#;

    println!("{:?}", MmlAst::parse(to_parse)?);
    Ok(())
}
