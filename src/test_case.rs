use proc_macro2::Ident;
use quote::quote;
use quote::ToTokens;
use syn::export::TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::parse_quote;
use syn::{Error, Expr, ItemFn, LitStr, Token};

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct TestCase {
    test_case_name: String,
    args: Vec<Expr>,
    expected: Option<Expr>,
    case_desc: Option<LitStr>,
}

fn fmt_syn(syn: &(impl ToTokens + Clone)) -> String {
    syn.clone().into_token_stream().to_string()
}

impl Parse for TestCase {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut test_case_name = String::new();

        let mut args = vec![];
        loop {
            let exp: Expr = input.parse()?;
            test_case_name += &format!(" {}", fmt_syn(&exp));
            args.push(exp);
            if !input.peek(Token![,]) {
                break;
            }
            let _comma: Token![,] = input.parse()?;
        }

        let arrow: Option<Token![=>]> = input.parse()?;

        let expected = if arrow.is_some() {
            let expr: Expr = input.parse()?;
            test_case_name += &format!(" expects {}", fmt_syn(&expr));
            Some(expr)
        } else {
            None
        };

        let semicolon: Option<Token![;]> = input.parse()?;
        let case_desc = if semicolon.is_some() {
            let desc: LitStr = input.parse()?;
            Some(desc)
        } else {
            None
        };

        Ok(Self {
            test_case_name,
            args,
            expected,
            case_desc,
        })
    }
}

impl TestCase {
    pub fn test_case_name(&self) -> Ident {
        let case_desc = self
            .case_desc
            .as_ref()
            .map(|cd| cd.value())
            .unwrap_or_else(|| self.test_case_name.clone());
        crate::utils::escape_test_name(case_desc)
    }

    pub fn render(&self, item: ItemFn) -> TokenStream2 {
        let item_name = item.sig.ident.clone();
        let arg_values = self.args.iter();
        let test_case_name = self.test_case_name();
        let inconclusive = self
            .case_desc
            .as_ref()
            .map(|cd| cd.value().to_lowercase().contains("inconclusive"))
            .unwrap_or_default();

        let expected: Expr = match &self.expected {
            Some(e) => parse_quote! {
                assert_eq!(#e, _result)
            },
            None => parse_quote! {()},
        };

        let mut attrs = vec![];
        if inconclusive {
            attrs.push(parse_quote! { #[ignore] });
        }
        attrs.append(&mut item.attrs.clone());

        quote! {
            #[test]
            #(#attrs)*
            fn #test_case_name() {
                let _result = #item_name(#(#arg_values),*);//{ #item_body };
                #expected
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_case {
        use super::*;

        mod parse {
            use super::*;
            use syn::parse_quote;

            #[test]
            fn parses_basic_input() {
                let actual: TestCase = parse_quote! {
                    2, 10
                };

                assert_eq!(
                    TestCase {
                        test_case_name: " 2 10".to_string(),
                        args: vec![
                            parse_quote!(2),
                            parse_quote!(10),
                        ],
                        expected: None,
                        case_desc: None,
                    },
                    actual
                );
            }

            #[test]
            fn parses_input_with_expectation() {
                let actual: TestCase = parse_quote! {
                    2, 10 => 12
                };

                assert_eq!(
                    TestCase {
                        test_case_name: " 2 10 expects 12".to_string(),
                        args: vec![
                            parse_quote!(2),
                            parse_quote!(10),
                        ],
                        expected: Some(parse_quote!(12)),
                        case_desc: None,
                    },
                    actual
                );
            }

            #[test]
            fn parses_input_with_description() {
                let actual: TestCase = parse_quote! {
                    2, 10; "basic addition"
                };

                assert_eq!(
                    TestCase {
                        test_case_name: " 2 10".to_string(),
                        args: vec![
                            parse_quote!(2),
                            parse_quote!(10),
                        ],
                        expected: None,
                        case_desc: parse_quote!("basic addition"),
                    },
                    actual
                );
            }
        }
    }
}
