extern crate proc_macro2;
extern crate syn;

use darling::{ast, FromDeriveInput, FromField, FromMeta};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};
use prometheus_exporter_base::prelude::*;

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

        let (imp, ty, wher) = generics.split_for_impl();

        // 
        let fields = data
            .as_ref()
            .take_struct()
            .expect("Shouldnt be an enum")
            .fields;


        // 
        let field_list = fields
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                if f.name.is_some() && f.metric_type.is_some() {
                    let field_name = f.name.as_ref().unwrap();

                    // Use an easy conversion from the `FromMeta` to a useable type
                    // If we figure out how to implement ToTokens for `MetricType`
                    // let field_metric_type: MetricType = f.metric_type.unwrap().into();

                    let field_metric_type = f.metric_type.unwrap();

                    let default_help = ::std::string::String::from("Generic help msg");
                    let field_help = f.help.as_ref().unwrap_or(&default_help);

                    let field_ident = f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                        let i = syn::Index::from(i);
                        quote!(#i)
                    });

                    // There's lots of duplicated code here, how fix?
                    // ToTokens is not implemented for `MetricType` so...
                    match field_metric_type {
                        PrometheusMetricType::Counter => quote! {
                            let mut #field_ident = PrometheusMetric::build()
                                .with_name(#field_name)
                                .with_metric_type(MetricType::from(field_metric_type))
                                .with_help(#field_help)
                                .build();
                            #field_ident.render_and_append_instance(
                                &PrometheusInstance::new()
                                    .with_value(self.#field_ident)
                            );
                            result.push(#field_ident.render());
                        },
                        PrometheusMetricType::Guage => quote! {
                            let mut #field_ident = PrometheusMetric::build()
                                .with_name(#field_name)
                                .with_metric_type(MetricType::from(field_metric_type))
                                .with_help(#field_help)
                                .build();
                            #field_ident.render_and_append_instance(
                                &PrometheusInstance::new()
                                    .with_value(self.#field_ident as usize)
                            );
                            result.push(#field_ident.render());
                        },
                        PrometheusMetricType::Text => quote! {
                            let mut #field_ident = PrometheusMetric::build()
                                .with_name(#field_name)
                                .with_metric_type(MetricType::Counter)
                                .with_help(#field_help)
                                .build();
                            #field_ident.render_and_append_instance(
                                &PrometheusInstance::new()
                                    .with_value(1usize)
                                    .with_label("value", ::std::string::String::from(self.#field_ident))
                            );
                            result.push(#field_ident.render());
                        },
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

                    #field_list

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

#[allow(unused)]
impl From<PrometheusMetricType> for MetricType {
    fn from(val: PrometheusMetricType) -> Self {
        match val {
            PrometheusMetricType::Counter => MetricType::Counter,
            PrometheusMetricType::Guage => MetricType::Gauge,
            PrometheusMetricType::Text => MetricType::Counter,
            _ => unimplemented!("This metric type is not yet supported.")
        }
    }
}
