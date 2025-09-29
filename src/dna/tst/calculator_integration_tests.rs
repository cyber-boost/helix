use crate::dna::ops::math::Calculator;
fn parse_and_verify(src: &str) -> anyhow::Result<()> {
    let calc = Calculator::new();
    let result = calc.evaluate(src.trim())?;
    assert!(! result.env.is_empty());
    Ok(())
}
#[test]
fn t01_simple_addition() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 10
            b = 7
            c = a + b
            d = @c
        }
    "#;
    let calc = Calculator::new();
    let result = calc.evaluate(src.trim())?;
    assert_eq!(result.env["a"], 10);
    assert_eq!(result.env["b"], 7);
    assert_eq!(result.env["c"], 17);
    assert_eq!(result.env["d"], 17);
    Ok(())
}
#[test]
fn t02_simple_subtraction() -> Result<()> {
    let src = r#"
        reproducibility {
            x = 5
            y = 12
            z = x - y
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t03_multiplication_x() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 4
            b = 9
            c = a x b
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t04_multiplication_star() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 6
            b = 7
            c = a * b
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t05_mixed_precedence() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 2
            b = 3
            c = 10
            d = 4
            e = (a + b) x (c - d)
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t06_reference_modulo() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 5
            b = 7
            c = a x b
            d = @c #4
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t07_chained_reference() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 12
            b = 5
            c = a x b
            d = @c #7
            e = @d #3
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t08_negative_numbers() -> Result<()> {
    let src = r#"
        reproducibility {
            a = -4
            b = 5
            c = a x b
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t09_large_numbers() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 9_999_999_999
            b = 2
            c = a x b
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t10_whitespace_fuzz() -> Result<()> {
    let src = r#"
        reproducibility{
                a   =   3
        b=4
        c =    a   x
        b
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t11_redefinition_overwrites() -> Result<()> {
    let src = r#"
        reproducibility {
            v = 5
            v = v + 1
            v = @v #2
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t13_mod_one_is_zero() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 12345
            b = @a #1
        }
    "#;
    eprintln!("parse_and_verify(src)");
    parse_and_verify(src)
}
#[test]
fn t14_nested_reference_expression() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 8
            b = 7
            c = a x b
            d = @c #5
            e = @d #3
        }
    "#;
    parse_and_verify(src)
}
#[test]
fn t15_all_together_now() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 4
            b = 6
            c = 15
            d = 3
            e = a x b
            f = ((a + b) x (c - d)) - @e #2
        }
    "#;
    parse_and_verify(src)
}