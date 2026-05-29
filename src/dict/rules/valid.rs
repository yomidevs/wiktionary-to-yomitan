//! This file was generated and should not be edited directly.
//! The source code can be found at scripts/deinflection_rules.py

use crate::lang::Lang;

#[rustfmt::skip]
pub fn is_valid_rule(lang: Lang, rule: &str) -> bool {
    match lang {
        Lang::Sq => matches!(rule, "adj" | "adv" | "n" | "np" | "ns" | "v"),
        Lang::Fr => matches!(rule, "adj" | "adv" | "aux" | "n" | "v"),
        Lang::Ga => matches!(rule, "adj" | "adv" | "n" | "np" | "ns" | "v" | "v_phr"),
        Lang::Ko => matches!(rule, "adj" | "do" | "euo" | "euob" | "eusi" | "f" | "ida" | "jab" | "jao" | "jaob" | "p" | "sab" | "sao" | "saob" | "v"),
        Lang::Es => matches!(rule, "adj" | "n" | "np" | "ns" | "v" | "v_ar" | "v_er" | "v_ir"),
        Lang::Eu => matches!(rule, "adj" | "adv" | "n" | "v"),
        Lang::Eo => matches!(rule, "adj" | "adv" | "n" | "v"),
        Lang::En => matches!(rule, "adj" | "adv" | "n" | "np" | "ns" | "v" | "v_phr"),
        Lang::Ja => matches!(rule, "-く" | "-た" | "-て" | "-なさい" | "-ば" | "-ます" | "-ません" | "-ゃ" | "-ん" | "adj-i" | "v" | "v1" | "v1d" | "v1p" | "v5" | "v5d" | "v5s" | "v5sp" | "v5ss" | "vk" | "vs" | "vz"),
        Lang::Yi => matches!(rule, "adj" | "adv" | "n" | "np" | "ns" | "v" | "vpast" | "vpresent"),
        Lang::La => matches!(rule, "adj" | "adj12" | "adj3" | "adv" | "n" | "n1" | "n1p" | "n1s" | "n2" | "n2p" | "n2s" | "n3" | "n3p" | "n3s" | "n4" | "n4p" | "n4s" | "n5" | "n5p" | "n5s" | "np" | "ns" | "v"),
        Lang::De => matches!(rule, "adj" | "n" | "v" | "vst" | "vw"),
        Lang::Ar => matches!(rule, "cv" | "cv_p" | "cv_s" | "iv" | "iv_p" | "iv_s" | "n" | "n_al" | "n_bi" | "n_bi_al" | "n_def" | "n_indef" | "n_ka" | "n_ka_al" | "n_li" | "n_li_al" | "n_lil" | "n_nom" | "n_nom_indef" | "n_p" | "n_s" | "n_wa" | "pv" | "pv_p" | "pv_s" | "v"),
        Lang::El => matches!(rule, "v"),
        Lang::Grc => matches!(rule, "adj" | "n" | "v"),
        _ => false,
    }
}
