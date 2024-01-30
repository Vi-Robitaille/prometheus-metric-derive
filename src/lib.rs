extern crate proc_macro2;
extern crate syn;

use proc_macro2::{Spacing, Span, Punct, TokenStream};
use darling::{ast, FromDeriveInput, FromField, FromMeta};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput, Ident};

// https://github.com/TedDriggs/darling/blob/master/examples/consume_fields.rs

#[proc_macro_derive(Metric, attributes(prometheus))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    let receiver = PromInputReciever::from_derive_input(&parsed).unwrap();
    let output = quote!(#receiver);

    output.into()
}

#[derive(Debug, FromDeriveInput)]
struct PromInputReciever {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), PromFieldReciever>,
}

impl ToTokens for PromInputReciever {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let PromInputReciever {
            ref ident,
            ref generics,
            ref data,
        } = *self;

        let (
            imp, 
            ty, 
            wher
        ) = generics.split_for_impl();

        //
        let fields = data
            .as_ref()
            .take_struct()
            .expect("Shouldnt be an enum")
            .fields;

        //
        let field_list_get_metrics = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if f.name.is_some() && f.metric_type.is_some() {
                    let field_name = f.name.as_ref().unwrap();

                    // Use an easy conversion from the `FromMeta` to a useable type
                    // If we figure out how to implement ToTokens for `MetricType`
                    // let field_metric_type: MetricType = f.metric_type.unwrap().into();

                    // as specified by `metric_type = "foo"`
                    let field_metric_type = f.metric_type.unwrap();

                    let default_help = ::std::string::String::from("Generic help msg");
                    let field_help = f.help.as_ref().unwrap_or(&default_help);

                    let field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                        let i = syn::Index::from(i);
                        quote!(#i)
                    });

                    // Get the conversion done here so we can generalize

                    let metric_type = quote!{ #field_metric_type };

                    let prometheus_instance = match field_metric_type {
                        PrometheusMetricType::Counter => quote! {
                            &PrometheusInstance::new()
                                    .with_value(self.#field_ident)
                        },
                        PrometheusMetricType::Guage => quote! {
                            &PrometheusInstance::new()
                                    .with_value(self.#field_ident as usize)
                        },
                        PrometheusMetricType::Text => quote! {
                            &PrometheusInstance::new()
                                    .with_value(1usize)
                                    .with_label("value", self.#field_ident.as_str())
                        },
                    };

                    quote! {
                        let mut #field_ident = PrometheusMetric::build()
                            .with_name(#field_name)
                            .with_metric_type(#metric_type)
                            .with_help(#field_help)
                            .build();
                        #field_ident.render_and_append_instance(
                            #prometheus_instance
                        );
                        result.push(#field_ident.render());
                    }
                } else {
                    quote! {}
                }
            })
            .fold(quote! {}, |acc, new| {
                quote! {
                    #acc
                    #new
                }
            });

            let field_list_get_metrics_with_prefix = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if f.name.is_some() && f.metric_type.is_some() {
                    let field_name = f.name.as_ref().unwrap();
    
                    // Use an easy conversion from the `FromMeta` to a useable type
                    // If we figure out how to implement ToTokens for `MetricType`
                    // let field_metric_type: MetricType = f.metric_type.unwrap().into();
    
                    // as specified by `metric_type = "foo"`
                    let field_metric_type = f.metric_type.unwrap();
    
                    let default_help = ::std::string::String::from("Generic help msg");
                    let field_help = f.help.as_ref().unwrap_or(&default_help);
    
                    let field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                        let i = syn::Index::from(i);
                        quote!(#i)
                    });
    
                    // Get the conversion done here so we can generalize
    
                    let metric_type = quote!{ #field_metric_type };
    
                    let prometheus_instance = match field_metric_type {
                        PrometheusMetricType::Counter => quote! {
                            &PrometheusInstance::new()
                                    .with_value(self.#field_ident)
                        },
                        PrometheusMetricType::Guage => quote! {
                            &PrometheusInstance::new()
                                    .with_value(self.#field_ident as usize)
                        },
                        PrometheusMetricType::Text => quote! {
                            &PrometheusInstance::new()
                                    .with_value(1usize)
                                    .with_label("value", self.#field_ident.as_str())
                        },
                    };
    
                    quote! {
                        let mut #field_ident = PrometheusMetric::build()
                            .with_name(format!("{}_{}", prefix, #field_name))
                            .with_metric_type(#metric_type)
                            .with_help(#field_help)
                            .build();
                        #field_ident.render_and_append_instance(
                            #prometheus_instance
                        );
                        result.push(#field_ident.render());
                    }
                } else {
                    quote! {}
                }
            })
            .fold(quote! {}, |acc, new| {
                quote! {
                    #acc
                    #new
                }
            });

        tokens.extend(quote! {
            impl #imp Metric for #ident #ty #wher {
                fn get_metrics(&self) -> ::std::string::String {
                    let mut result: ::std::vec::Vec<::std::string::String> = Vec::new();

                    #field_list_get_metrics

                    result.concat()
                }

                fn get_metrics(&self, prefix: &str) -> ::std::string::String {
                    let mut result: ::std::vec::Vec<::std::string::String> = Vec::new();

                    #field_list_get_metrics_with_prefix

                    result.concat()
                }
            }
        });
    }
}

#[allow(unused)]
#[derive(Debug, FromField)]
#[darling(attributes(prometheus))]
struct PromFieldReciever {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    name: Option<String>,
    metric_type: Option<PrometheusMetricType>,
    help: Option<String>,
    // extra_labels: Option<(String, String)>,
}

/// A metric type in prometheus.
/// Deriving `FromMeta` will cause this to be usable
/// as a string value for a meta-item key.
#[derive(Debug, Clone, Copy, FromMeta)]
enum PrometheusMetricType {
    Counter,
    Guage,
    Text,
}

impl quote::ToTokens for PrometheusMetricType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("MetricType", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Alone));
        let t = match self {
            PrometheusMetricType::Counter => Ident::new("Counter", Span::call_site()),
            PrometheusMetricType::Guage => Ident::new("Gauge", Span::call_site()),
            PrometheusMetricType::Text => Ident::new("Counter", Span::call_site()),
        };
        tokens.append(t);
    }
}
