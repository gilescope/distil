#![feature(slice_group_by)]
//#![feature(drain_filter)]
//#![feature(slice_pattern)]
//use core::slice::SlicePattern;
use emojis::Emoji;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::io::BufRead;

/// World's least efficient program
fn main() {
    let stdin = std::io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map(|l| l.unwrap()).collect();
    println!("{}", compress(lines.join("\n")));
}

fn distil(input: &[u8], max_suggestions: usize) -> Vec<String> {
    const WINDOW_SIZE: usize = 5;
    let mut freq = HashMap::<[u8; WINDOW_SIZE], u32>::new();
    let inp = input.windows(WINDOW_SIZE);

    for i in inp {
        let entry = freq.entry(i.try_into().unwrap()).or_insert(0);
        *entry += 1;
    }

    let mut res = vec![];

    for _ in 0..max_suggestions {
        let max = freq.values().max().cloned();
        if let Some(max) = max {
            // reverse lookup: find max seed pattern.
            let mut seek: [u8; WINDOW_SIZE] = *b"01234";
            for (k, v) in &freq {
                if *v == max {
                    seek = *k;
                    break;
                }
            }
            freq.remove(&seek);

            let inp = input;
            let mut starts = Vec::<usize>::new();
            for i in 0..(inp.len() - WINDOW_SIZE) {
                if inp[i..i + WINDOW_SIZE] == seek {
                    starts.push(i);
                }
            }

            //TODO: if start INSIDE previous suggestion then continue!

            let grown = grow(WINDOW_SIZE, &starts, input);

            if let Some((len, points)) = grown {
                res.push(String::from_utf8_lossy(&input[points[0]..points[0] + len]).to_string())
            }
        } else {
            break;
        }
    }
    res
}

fn grow(current_len: usize, og_groups: &[usize], input: &[u8]) -> Option<(usize, Vec<usize>)> {
    let res = grow_forwards(current_len, og_groups, input);
    if let Some((len, groups)) = res {
        grow_backwards(len, groups.as_slice(), input)
    } else {
        grow_backwards(current_len, og_groups, input)
    }
}

/// Provide a larger
/// current_len is the length of the common portion found so far.
/// input is the haystack to look in to consider extending.
/// groups is the indexs where the common lengths begin.
fn grow_forwards(
    current_len: usize,
    og_groups: &[usize],
    input: &[u8],
) -> Option<(usize, Vec<usize>)> {
    if og_groups.len() < 2 {
        return None;
    }
    // check if we've hit the end of the input:
    let last = og_groups[og_groups.len() - 1];
    if last + current_len >= input.len() {
        //At the moment instafail this but in reality we are truncating some results.
        return None;
    }

    // Sort
    let mut group: Vec<_> = og_groups
        .iter()
        .map(|&i| (i, input[i + current_len]))
        .collect();
    group.sort_unstable_by_key(|(_, ch)| *ch);

    let groups = group.group_by(|(_, cha), (_, chb)| cha == chb);

    let mut best_len = current_len;
    let mut groups_to_beat: Vec<usize> = og_groups.to_vec();
    let mut vol_to_beat = best_len * groups_to_beat.len();

    for idxs in groups {
        let idxs: Vec<usize> = idxs.iter().map(|(i, _)| *i).collect();
        if idxs.len() < 2 {
            continue;
        } //todo: overlaps???
        let grewed = grow_forwards(current_len + 1, idxs.as_slice(), input);
        if let Some((grewed, grew_groups)) = grewed {
            let new_vol = grewed * grew_groups.len();
            if new_vol > vol_to_beat {
                vol_to_beat = new_vol;
                best_len = grewed;
                groups_to_beat = grew_groups;
            }
        }
    }

    Some((best_len, groups_to_beat))
}

/// assuming group indicies are sorted smallest to largests.
fn grow_backwards(
    current_len: usize,
    og_groups: &[usize],
    input: &[u8],
) -> Option<(usize, Vec<usize>)> {
    if og_groups.len() < 2 {
        return None;
    }
    // check if we've hit the end of the input:
    let first = og_groups[0];
    if first == 0 {
        //At the moment instafail this but in reality we are truncating some results.
        return None;
    }

    // Sort
    let mut group: Vec<_> = og_groups.iter().map(|&i| (i, input[i - 1])).collect();
    group.sort_unstable_by_key(|(_, ch)| *ch);

    let groups = group.group_by(|(_, cha), (_, chb)| cha == chb);

    let mut best_len = current_len;
    let mut groups_to_beat: Vec<usize> = og_groups.to_vec();
    let mut vol_to_beat = best_len * groups_to_beat.len();

    for idxs in groups {
        let idxs: Vec<usize> = idxs.iter().map(|(i, _)| *i).collect();
        if idxs.len() < 2 {
            continue;
        } //todo: overlaps???
        let grewed = grow_backwards(
            current_len + 1,
            idxs.iter()
                .map(|x| x - 1)
                .collect::<Vec<usize>>()
                .as_slice(),
            input,
        );
        if let Some((grewed, grew_groups)) = grewed {
            let new_vol = grewed * grew_groups.len();
            if new_vol > vol_to_beat {
                vol_to_beat = new_vol;
                best_len = grewed;
                groups_to_beat = grew_groups;
            }
        }
    }

    Some((best_len, groups_to_beat))
}

fn compress(a: String) -> String {
    let mut results = String::new();
    let mut emojis_used = HashSet::new();
    let res = compress_aux(a, &mut results, &mut emojis_used);
    if emojis_used.is_empty() {
        return res;
    }
    return format!("{}\n where {}", res, &results);
}

fn compress_aux(input: String, out: &mut String, used: &mut HashSet<Emoji>) -> String {
    if input.len() < 5 {
        return input;
    }

    let it = distil(strip_ansi_escapes::strip(&input).unwrap().as_slice(), 40);
    let mut res = input;
    for i in it {
        let m = i;
        if !m.contains('/') && !m.contains('^') {
            //don't mess with paths

            if m.len() > 0 {
                let mut replace = m.trim().to_string();
                if replace.ends_with("::") {
                    replace.pop();
                    replace.pop();
                }
                replace = trim_bracket(&replace).trim().to_string();
                const UNICODE_CHAR_LEN: usize = 4;
                if replace.len() > UNICODE_CHAR_LEN {
                    let emo = pick_subset(&replace, used);
                    used.insert(emo.clone());
                    //                    println!("replace '{}', '{}'", &replace, emo.as_str());

                    let res_new = res.replace(&replace, emo.as_str());
                    if res != res_new {
                        out.push_str(&format!(
                            "\n{} ({})\t= {}",
                            emo.as_str(),
                            emo.shortcode().unwrap(),
                            replace
                        ));
                        res = res_new;
                    }
                }
            }
        }
    }
    return res;
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    }

    if max_closure_span.1 - max_closure_span.0 > 0 {
        return &big[max_closure_span.0..max_closure_span.1];
    }

    return big;
}

//use bcmp::{AlgoSpec, MatchIterator};
#[test]
fn test_real() {
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

    //    let (b, c) = a.split_at(a.len() / 2);

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

#[test]
fn ok_her() {
    let s = include_str!("/Users/bit/p/substrate/test-utils/runtime/my.txt");
    let y = compress(s.to_string());
    println!("{}", y);
}

#[test]
fn grpw_it_forwards() {
    //hit end
    assert_eq!(grow_forwards(2, &[0, 3], b"abcabc"), Some((2, vec![0, 3])));

    assert_eq!(grow_forwards(2, &[0, 3], b"abcabcX"), Some((3, vec![0, 3])));

    assert_eq!(
        grow_forwards(2, &[0, 3, 6], b"abcabcabcX"),
        Some((3, vec![0, 3, 6]))
    );
}

#[test]
fn grpw_it_backwards() {
    assert_eq!(
        grow_backwards(1, &[3, 6], b" abcabc"),
        Some((3, vec![1, 4]))
    );
}

#[test]
fn grow_it() {
    assert_eq!(grow(1, &[2, 6], b" abc abc "), Some((3, vec![1, 5])));
}
