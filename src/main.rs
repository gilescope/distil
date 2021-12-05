//#![feature(slice_group_by)]
//#![feature(drain_filter)]
use emojis::Emoji;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::BufRead;

/// World's least efficient program
fn main() {
    let stdin = std::io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map(|l| l.unwrap()).collect();
    println!("{}", compress(lines.join("\n")));
}

// fn distil(input: &str, ledgend: &str) -> Vec<String> {
//     const window_size: usize = 5;
//     let mut freq = HashMap::<[u8; window_size], u32>::new();
//     let inp = input.as_bytes().windows(window_size);
//     let min_number = 4;
//     let result = String::new();

//     for i in inp {
//         let entry = freq.entry(i.try_into().unwrap()).or_insert(0);
//         *entry += 1;
//     }

//     let max = *freq.values().max().unwrap();
//     // for (k, v) in freq {
//     println!("max = {}", max);
//     let mut seek: [u8; window_size] = *b"01234";
//     for (k, v) in freq {
//         if v == max {
//             println!("max = {}", String::from_utf8_lossy(&k));
//             seek = k;
//             break;
//         }
//     }

//     //grow k

//     let inp = input.as_bytes();
//     let mut start_end = Vec::<(usize, usize)>::new();
//     for i in 0..(inp.len() - window_size) {
//         if inp[i..i + window_size] == seek {
//             start_end.push((i, i + window_size));
//         }
//     }
//     println!("occurances: {}", start_end.len());
//     'outer: loop {
//         // look at all next char
//         let groups: Vec<u8> = start_end.iter().map(|(_, end)| inp[end + 1]).collect();
//         //.sort()

//         let mut changed = false;
//         let mut ch = None;
//         for group in groups.group_by(|x, y| x == y) {
//             if group.len() > min_number {
//                 ch = Some(group[0]);
//                 for (s, e) in &mut start_end {
//                     *e += 1;
//                 }
//                 changed = true;
//                 println!("extended");

//                 break;
//             }
//         }

//         if !changed {
//             break;
//         }

//         let ch = ch.unwrap();

//         //discard ones that don't match
//         start_end.drain_filter(|(s, e)| inp[*e + 1] != ch);
//     }

//     let (s, e) = start_end[0];
//     vec![String::from_utf8_lossy(&inp[s..e]).to_owned().to_string()]
// }

fn compress(a: String) -> String {
    let mut results = String::from("\nwhere");
    let mut emojis_used = HashSet::new();
    let res = compress_aux(a, &mut results, &mut emojis_used);
    if emojis_used.is_empty() {
        return res;
    }
    return format!("{}\n where {}", res, &results);
}

fn compress_aux(a: String, out: &mut String, used: &mut HashSet<Emoji>) -> String {
    if a.len() < 5 {
        return a;
    }
    let count = a.chars().count();
    let (ch_idx, _chr) = a.char_indices().skip(count / 2).next().unwrap();
    let (b, c) = a.split_at(ch_idx);
    {
        let mut m = String::new();
        let it = MatchIterator::new(b.as_bytes(), c.as_bytes(), AlgoSpec::HashMatch(10));
        for i in it {
            if i.first_end() - i.first_pos > m.len() {
                if let Ok(v) = std::str::from_utf8(&a.as_bytes()[i.first_pos..i.first_end()]) {
                    if !v.contains('/') {
                        //don't mess with paths
                        m = v.to_string();
                    }
                    break;
                }
            }
        }
        if m.len() > 0 {
            let mut replace = m.trim().to_string();
            if replace.ends_with("::") {
                replace.pop();
                replace.pop();
            }
            replace = trim_bracket(&replace).trim().to_string();
            const UNICODE_CHAR_LEN: usize = 4;
            if replace.len() > UNICODE_CHAR_LEN {
                // let s = emojis::use emojis::Emoji;iter().count();
                // println!("totoooasd {}", s);

                // let mut hasher = DefaultHasher::new();
                // replace.hash(&mut hasher);
                // let emoji_num = (hasher.finish() % 100) as usize;
                // let emo = emojis::iter()
                //     .skip(emoji_num + 200)
                //     .next()
                //     .unwrap()
                //     .as_str()
                //     .chars()
                //     .next()
                //     .unwrap();

                let emo = pick_subset(&replace, used);
                used.insert(emo.clone());
                // let emog = emo.as_str().chars().next().unwrap();
                let replace = String::from_utf8_lossy(
                    strip_ansi_escapes::strip(&replace).unwrap().as_slice(),
                )
                .to_string();
                let mut res = a.replace(&replace, emo.as_str());
                if res != a {
                    out.push_str(&format!(
                        "\n{} ({})\t= {}",
                        emo.as_str(),
                        emo.shortcode().unwrap(),
                        replace
                    ));
                    res = compress_aux(res, out, used);
                    return res;
                }
            }
        }
    }
    return a;
}

fn pick_random(replace: &str) -> char {
    let mut hasher = DefaultHasher::new();
    replace.hash(&mut hasher);
    let emoji_num = (hasher.finish() % 100) as usize;
    let emo = emojis::iter()
        .skip(emoji_num + 200)
        .next()
        .unwrap()
        .as_str()
        .chars()
        .next()
        .unwrap();
    emo
}

fn pick_distance(replace: &str) -> &'static Emoji {
    let (emo, _) = emojis::iter()
        .filter(|e| e.shortcode().is_some())
        .map(|e| {
            (
                e,
                edit_distance::edit_distance(e.shortcode().unwrap(), replace),
            )
        })
        .min_by_key(|&(_e, dist)| dist)
        .unwrap();

    emo
}

fn pick_subset(replace: &str, used: &mut HashSet<Emoji>) -> &'static Emoji {
    let lower = replace.to_lowercase();
    let mut best = None;
    let mut best_len = 0;
    for e in emojis::iter() {
        if let Some(short) = e.shortcode() {
            for part in short.split("_") {
                if part.len() > best_len {
                    if lower.contains(part) && !used.contains(e) {
                        best = Some(e);
                        best_len = part.len();
                    }
                }
            }
        }
    }

    best.unwrap_or(emojis::iter().next().unwrap())
}

/// Trim a string so that brackets are level.
/// Either too many open brackets <<< > or <> >>
fn trim_bracket(big: &str) -> &str {
    let directions = big
        .as_bytes()
        .iter()
        .map(|ch| match ch {
            b'<' => 1,
            b'>' => -1,
            _ => 0,
        })
        .enumerate();
    //let count = 0;

    let mut begin_idx = vec![];
    let mut max_closure_span = (0_usize, 0_usize);
    for (i, sign) in directions {
        if sign == 1 {
            begin_idx.push(i);
        }
        if sign == -1 {
            let val = begin_idx.pop();
            if let Some(val) = val {
                if max_closure_span.1 - max_closure_span.0 < i - val {
                    max_closure_span = (val + 1, i);
                }
            }
        }
        // if sign == -1 && count <= 0 {
        //     return trim_bracket(&big[i + 1..]);
        // }
        // if sign == -1 && count == 1 {
        //     return &big[..i + 1];
        // }
        //        count += sign;
    }

    if max_closure_span.1 - max_closure_span.0 > 0 {
        return &big[max_closure_span.0..max_closure_span.1];
    }

    //    if count == 0 {
    //        return big;
    // } else if count < 0 {
    //     // Too many closing brackets
    //     let mut idx = big.len() - 1;
    //     while count < 0 && idx >= 0 {
    //         if big.as_bytes()[idx] == b'>' {
    //             count += 1;
    //             if count == 0 {
    //                 //maybe we should go back one more excluding
    //                 return &big[..idx];
    //             }
    //         }
    //         idx -= 1;
    //     }
    // } else {
    //     // too many opening brackets
    //     let mut idx = 0;
    //     while count > 0 && idx < big.len() {
    //         if big.as_bytes()[idx] == b'<' {
    //             count -= 1;
    //             if count == 0 {
    //                 //maybe we should go back one more excluding
    //                 return &big[idx..];
    //             }
    //         }
    //         idx += 1;
    //     }
    // }
    return big;
}

use bcmp::{AlgoSpec, MatchIterator};
#[test]
fn test() {
    //     // error[E0277]: the trait bound `RuntimeApiImpl<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>>: GrandpaApi<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` is not satisfied
    //   --> test/service/src/lib.rs:175:16
    //     |
    // 175 |         import_queue(boxi, &task_manager.spawn_essential_handle(), None);
    //     |                      ^^^^ the trait `GrandpaApi<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` is not implemented for `RuntimeApiImpl<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>>`
    //     |
    //     = note: required because of the requirements on the impl of `BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` for `GrandpaBlockImport<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, sc_consensus::LongestChain<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>>`
    //     = note: 1 redundant requirements hidden
    //     = note: required because of the requirements on the impl of `BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` for `BabeBlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, GrandpaBlockImport<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, sc_consensus::LongestChain<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>>>`
    //     = note: required for the cast to the object type `dyn BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, Error = substrate_test_client::sp_consensus::Error, Transaction = memory_db::MemoryDB<BlakeTwo256, memory_db::PrefixedKey<BlakeTwo256>, Vec<u8>, memory_db::malloc_size_of::NoopTracker<Vec<u8>>>> + Sync + std::marker::Send`

    let a = r#"
    Checking cumulus-test-service v0.1.0 (/home/gilescope/git/substrate3/test/service)
warning: unused imports: `BabeConsensusDataProvider`, `EngineCommand`, `ManualSealApi`, `ManualSealParams`, `ManualSeal`, `SlotTimestampProvider`, `run_manual_seal`
  --> test/service/src/lib.rs:38:20
   |
38 |     consensus::babe::{BabeConsensusDataProvider, SlotTimestampProvider},
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^
39 |     import_queue,
40 |     rpc::{ManualSeal, ManualSealApi},
   |           ^^^^^^^^^^  ^^^^^^^^^^^^^
41 |     run_manual_seal, EngineCommand, ManualSealParams,
   |     ^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` on by default

warning: unused import: `sc_client_api::ExecutorProvider`
  --> test/service/src/lib.rs:68:5
   |
68 | use sc_client_api::ExecutorProvider;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0277]: the trait bound `RuntimeApiImpl<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>>: GrandpaApi<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` is not satisfied
   --> test/service/src/lib.rs:175:16
    |
175 |         import_queue(boxi, &task_manager.spawn_essential_handle(), None);
    |                      ^^^^ the trait `GrandpaApi<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` is not implemented for `RuntimeApiImpl<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>>`
    |
    = note: required because of the requirements on the impl of `BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` for `GrandpaBlockImport<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, sc_consensus::LongestChain<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>>`
    = note: 1 redundant requirements hidden
    = note: required because of the requirements on the impl of `BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` for `BabeBlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, GrandpaBlockImport<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>, sc_consensus::LongestChain<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>>>`
    = note: required for the cast to the object type `dyn BlockImport<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, Error = substrate_test_client::sp_consensus::Error, Transaction = memory_db::MemoryDB<BlakeTwo256, memory_db::PrefixedKey<BlakeTwo256>, Vec<u8>, memory_db::malloc_size_of::NoopTracker<Vec<u8>>>> + Sync + std::marker::Send`

warning: unused import: `sp_api::BlockT`
  --> test/service/src/lib.rs:55:5
   |
55 | use sp_api::BlockT;
   |     ^^^^^^^^^^^^^^

For more information about this error, try `rustc --explain E0277`.    
    // "#;

    let (b, c) = a.split_at(a.len() / 2);

    //let it = MatchIterator::new(b.as_bytes(), c.as_bytes(), AlgoSpec::HashMatch(10));
    //for m in it {
    //    println!("{:}", &a[m.first_pos..m.first_end()]);
    //}

    println!("{}", compress(a.to_string()));

    // distil(
    //     r#"
    // error[E0277]: the trait bound `RuntimeApiImpl<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, sc_service::client::Client<substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, LocalCallExecutor<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, substrate_test_client::Backend<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>, NativeElseWasmExecutor<RuntimeExecutor>>, sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>, cumulus_test_runtime::RuntimeApi>>: GrandpaApi<sp_runtime::generic::Block<sp_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>>` is not satisfied

    // "#,
    //     "💖😂🚀😜🚨",
    // );
}

#[test]
fn test_trim() {
    let s = r#"_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>"#;
    assert_eq!(trim_bracket(s), "u32, BlakeTwo256")
}

#[test]
fn test_trim_leading() {
    let s = r#">>>_runtime::generic::Header<u32, BlakeTwo256>, OpaqueExtrinsic>"#;
    assert_eq!(trim_bracket(s), "u32, BlakeTwo256")
}

#[test]
fn finds_biggest_closure() {
    let s = r#"<Backend<sp_runtime> Block<BiggggOpaqueExtrinsic<small>>"#;
    assert_eq!(trim_bracket(s), "BiggggOpaqueExtrinsic<small>")
}
