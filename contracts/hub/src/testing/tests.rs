use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DistributionMsg, Event, Order,
    OwnedDeps, Reply, ReplyOn, StdError, SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use std::str::FromStr;

use osmo_bindings::OsmosisMsg;
use steak::hub::{
    Batch, CallbackMsg, ConfigResponse, ExecuteMsg, InstantiateMsg, PendingBatch, QueryMsg,
    StateResponse, UnbondRequest, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem,
};

use crate::contract::{execute, instantiate, reply};
use crate::error::ContractError;
use crate::helpers::{parse_coin, parse_received_fund};
use crate::math::{
    compute_redelegations_for_rebalancing, compute_redelegations_for_removal, compute_undelegations,
};
use crate::state::State;
use crate::types::{Coins, Delegation, Redelegation, Undelegation};

use super::custom_querier::CustomQuerier;
use super::helpers::{mock_dependencies, mock_env_at_timestamp, query_helper};
const DENOM: &str = "factory/cosmos2contract/apOSMO";

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

fn setup_test() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    let mut deps = mock_dependencies();

    let _res = instantiate(
        deps.as_mut(),
        mock_env_at_timestamp(10000),
        mock_info("deployer", &[]),
        InstantiateMsg {
            owner: "apollo".to_string(),
            name: "apOSMO".to_string(),
            symbol: "apOSMO".to_string(),
            decimals: 6,
            epoch_period: 259200,   // 3 * 24 * 60 * 60 = 3 days
            unbond_period: 1814400, // 21 * 24 * 60 * 60 = 21 days
            validators: vec![
                "alice".to_string(),
                "bob".to_string(),
                "charlie".to_string(),
            ],
            performance_fee: 5,
            distribution_contract: "distribution_contract".to_string(),
        },
    )
    .unwrap();

    deps
}
//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn proper_instantiation() {
    let deps = setup_test();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            owner: "apollo".to_string(),
            new_owner: None,
            steak_denom: DENOM.to_string(),
            epoch_period: 259200,
            unbond_period: 1814400,
            validators: vec![
                "alice".to_string(),
                "bob".to_string(),
                "charlie".to_string()
            ],
            performance_fee: Decimal::percent(5),
            distribution_contract: Addr::unchecked("distribution_contract")
        }
    );

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_usteak: Uint128::zero(),
            total_uosmo: Uint128::zero(),
            exchange_rate: Decimal::one(),
            unlocked_coins: vec![],
        },
    );

    let res: PendingBatch = query_helper(deps.as_ref(), QueryMsg::PendingBatch {});
    assert_eq!(
        res,
        PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: 269200, // 10,000 + 259,200
        },
    );
}

#[test]
fn bonding() {
    let mut deps = setup_test();
    let state = State::default();

    // Bond when no delegation has been made
    // In this case, the full deposit simply goes to the first validator
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_1", &[Coin::new(1000000, "uosmo")]),
        ExecuteMsg::Bond { receiver: None },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(Delegation::new("alice", 1000000).to_cosmos_msg(), 1)
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Custom(OsmosisMsg::MintTokens {
                denom: DENOM.to_string(),
                amount: Uint128::new(1000000),
                mint_to_address: "user_1".to_string()
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );

    // Bond when there are existing delegations, and OSMO:Steak exchange rate is >1
    // Previously user 1 delegated 1,000,000 uosmo. We assume we have accumulated 2.5% yield at 1025000 staked
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);

    state
        .total_usteak_supply
        .save(&mut deps.storage, &Uint128::new(1000000u128))
        .unwrap();

    // Charlie has the smallest amount of delegation, so the full deposit goes to him
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_2", &[Coin::new(12345, "uosmo")]),
        ExecuteMsg::Bond {
            receiver: Some("user_3".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(Delegation::new("charlie", 12345).to_cosmos_msg(), 1)
    );
    assert_eq!(
        res.messages[1],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Custom(OsmosisMsg::MintTokens {
                denom: DENOM.to_string(),
                amount: Uint128::new(12043),
                mint_to_address: "user_3".to_string()
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // Check the state after bonding
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 354011),
    ]);
    state
        .total_usteak_supply
        .save(&mut deps.storage, &Uint128::new(1012043u128))
        .unwrap();

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_usteak: Uint128::new(1012043),
            total_uosmo: Uint128::new(1037345),
            exchange_rate: Decimal::from_ratio(1037345u128, 1012043u128),
            unlocked_coins: vec![],
        }
    );
}

#[test]
fn harvesting() {
    let mut deps = setup_test();

    let state = State::default();
    // Assume users have bonded a total of 1,000,000 uosmo and minted the same amount of usteak
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);
    state
        .total_usteak_supply
        .save(&mut deps.storage, &Uint128::new(1000000))
        .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("worker", &[]),
        ExecuteMsg::Harvest {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: "alice".to_string(),
            }),
            1,
        )
    );
    assert_eq!(
        res.messages[1],
        SubMsg::reply_on_success(
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: "bob".to_string(),
            }),
            1,
        )
    );
    assert_eq!(
        res.messages[2],
        SubMsg::reply_on_success(
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: "charlie".to_string(),
            }),
            1,
        )
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Reinvest {})).unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );
}

#[test]
fn registering_unlocked_coins() {
    let mut deps = setup_test();
    let state = State::default();

    // After withdrawing staking rewards, we parse the `coin_received` event to find the received amounts
    let event = Event::new("coin_received")
        .add_attribute("receiver", MOCK_CONTRACT_ADDR.to_string())
        .add_attribute("amount", "123ukrw,234uosmo,345uusd,69420ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B");

    reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![event],
                data: None,
            }),
        },
    )
    .unwrap();

    // Unlocked coins in contract state should have been updated
    let unlocked_coins = state.unlocked_coins.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        unlocked_coins,
        vec![
            Coin::new(123, "ukrw"),
            Coin::new(234, "uosmo"),
            Coin::new(345, "uusd"),
            Coin::new(
                69420,
                "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"
            ),
        ]
    );
}

#[test]
fn reinvesting() {
    let mut deps = setup_test();
    let state = State::default();

    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 333334),
        Delegation::new("bob", 333333),
        Delegation::new("charlie", 333333),
    ]);

    // After the swaps, `unlocked_coins` should contain only uosmo and unknown denoms
    state
        .unlocked_coins
        .save(
            deps.as_mut().storage,
            &vec![
                Coin::new(234, "uosmo"),
                Coin::new(
                    69420,
                    "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B",
                ),
            ],
        )
        .unwrap();

    // Bob has the smallest amount of delegations, so all proceeds go to him
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackMsg::Reinvest {}),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages,
        vec![
            SubMsg {
                id: 0,
                msg: Delegation::new("bob", 222).to_cosmos_msg(),
                gas_limit: None,
                reply_on: ReplyOn::Never
            },
            SubMsg {
                id: 0,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "distribution_contract".to_string(),
                    amount: coins(12u128, "uosmo")
                }),
                gas_limit: None,
                reply_on: ReplyOn::Never
            }
        ]
    );

    // Storage should have been updated
    let unlocked_coins = state.unlocked_coins.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        unlocked_coins,
        vec![Coin::new(
            69420,
            "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"
        )],
    );
}

#[test]
fn queuing_unbond() {
    let mut deps = setup_test();
    let state = State::default();

    // Only Steak token is accepted for unbonding requests
    let mut err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("hacker", &[]),
        ExecuteMsg::QueueUnbond { receiver: None },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::NoCoinsSent {});

    err = execute(
        deps.as_mut(),
        mock_env_at_timestamp(12345), // est_unbond_start_time = 269200
        mock_info("user_1", &[coin(1000u128, "random")]),
        ExecuteMsg::QueueUnbond { receiver: None },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::InvalidCoinSent {});

    // User 1 creates an unbonding request before `est_unbond_start_time` is reached. The unbond
    // request is saved, but not the pending batch is not submitted for unbonding
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(12345), // est_unbond_start_time = 269200
        mock_info("user_1", &[coin(23456u128, DENOM)]),
        ExecuteMsg::QueueUnbond { receiver: None },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    // User 3 creates an unbonding request after `est_unbond_start_time` is reached. The unbond
    // request is saved, and the pending is automatically submitted for unbonding
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(269201), // est_unbond_start_time = 269200
        mock_info("user_3", &[coin(69420u128, DENOM)]),
        ExecuteMsg::QueueUnbond { receiver: None },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::SubmitBatch {}).unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // The users' unbonding requests should have been saved
    let ubr1 = state
        .unbond_requests
        .load(
            deps.as_ref().storage,
            (1u64.into(), &Addr::unchecked("user_1")),
        )
        .unwrap();
    let ubr2 = state
        .unbond_requests
        .load(
            deps.as_ref().storage,
            (1u64.into(), &Addr::unchecked("user_3")),
        )
        .unwrap();

    assert_eq!(
        ubr1,
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456)
        }
    );
    assert_eq!(
        ubr2,
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420)
        }
    );

    // Pending batch should have been updated
    let pending_batch = state.pending_batch.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        pending_batch,
        PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::new(92876), // 23,456 + 69,420
            est_unbond_start_time: 269200
        }
    );
}

#[test]
fn submitting_batch() {
    let mut deps = setup_test();
    let state = State::default();

    // uosmo bonded: 1,037,345
    // usteak supply: 1,012,043
    // uosmo per ustake: 1.025
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 345782),
        Delegation::new("bob", 345782),
        Delegation::new("charlie", 345781),
    ]);

    state
        .total_usteak_supply
        .save(&mut deps.storage, &Uint128::new(1012043u128))
        .unwrap();

    // We continue from the contract state at the end of the last test
    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (
                    unbond_request.id.into(),
                    &Addr::unchecked(unbond_request.user.clone()),
                ),
                unbond_request,
            )
            .unwrap();
    }

    state
        .pending_batch
        .save(
            deps.as_mut().storage,
            &PendingBatch {
                id: 1,
                usteak_to_burn: Uint128::new(92876), // 23,456 + 69,420
                est_unbond_start_time: 269200,
            },
        )
        .unwrap();

    // Anyone can invoke `submit_batch`. Here we continue from the previous test and assume it is
    // invoked automatically as user 2 submits the unbonding request
    //
    // usteak to burn: 23,456 + 69,420 = 92,876
    // uosmo to unbond: 1,037,345 * 92,876 / 1,012,043 = 95,197
    //
    // Target: (1,037,345 - 95,197) / 3 = 314,049
    // Remainer: 1
    // Alice:   345,782 - (314,049 + 1) = 31,732
    // Bob:     345,782 - (314,049 + 0) = 31,733
    // Charlie: 345,781 - (314,049 + 0) = 31,732
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(269201),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::SubmitBatch {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(Undelegation::new("alice", 31732).to_cosmos_msg(), 1)
    );
    assert_eq!(
        res.messages[1],
        SubMsg::reply_on_success(Undelegation::new("bob", 31733).to_cosmos_msg(), 1)
    );
    assert_eq!(
        res.messages[2],
        SubMsg::reply_on_success(Undelegation::new("charlie", 31732).to_cosmos_msg(), 1)
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Custom(OsmosisMsg::BurnTokens {
                denom: DENOM.to_string(),
                amount: Uint128::new(92876),
                burn_from_address: "".to_string()
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // A new pending batch should have been created
    let pending_batch = state.pending_batch.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        pending_batch,
        PendingBatch {
            id: 2,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: 528401 // 269,201 + 259,200
        }
    );

    // Previous batch should have been updated
    let previous_batch = state
        .previous_batches
        .load(deps.as_ref().storage, 1u64.into())
        .unwrap();
    assert_eq!(
        previous_batch,
        Batch {
            id: 1,
            reconciled: false,
            total_shares: Uint128::new(92876),
            uosmo_unclaimed: Uint128::new(95197),
            est_unbond_end_time: 2083601 // 269,201 + 1,814,400
        }
    );
}

#[test]
fn reconciling() {
    let mut deps = setup_test();
    let state = State::default();

    let previous_batches = vec![
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(92876),
            uosmo_unclaimed: Uint128::new(95197), // 1.025 OSMO per Steak
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: false,
            total_shares: Uint128::new(1345),
            uosmo_unclaimed: Uint128::new(1385), // 1.030 OSMO per Steak
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 3,
            reconciled: false,
            total_shares: Uint128::new(1456),
            uosmo_unclaimed: Uint128::new(1506), // 1.035 OSMO per Steak
            est_unbond_end_time: 30000,
        },
        Batch {
            id: 4,
            reconciled: false,
            total_shares: Uint128::new(1567),
            uosmo_unclaimed: Uint128::new(1629), // 1.040 OSMO per Steak
            est_unbond_end_time: 40000,          // not yet finished unbonding, ignored
        },
    ];

    for previous_batch in &previous_batches {
        state
            .previous_batches
            .save(
                deps.as_mut().storage,
                previous_batch.id.into(),
                previous_batch,
            )
            .unwrap();
    }

    state
        .unlocked_coins
        .save(
            deps.as_mut().storage,
            &vec![
                Coin::new(10000, "uosmo"),
                Coin::new(234, "ukrw"),
                Coin::new(345, "uusd"),
                Coin::new(
                    69420,
                    "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B",
                ),
            ],
        )
        .unwrap();

    deps.querier.set_bank_balances(&[
        Coin::new(12345, "uosmo"),
        Coin::new(234, "ukrw"),
        Coin::new(345, "uusd"),
        Coin::new(
            69420,
            "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B",
        ),
    ]);

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(35000),
        mock_info("worker", &[]),
        ExecuteMsg::Reconcile {},
    )
    .unwrap();

    // Expected received: batch 2 + batch 3 = 1385 + 1506 = 2891
    // Expected unlocked: 10000
    // Expected: 12891
    // Actual: 12345
    // Shortfall: 12891 - 12345 = 456
    //
    // uosmo per batch: 546 / 2 = 273
    // remainder: 0
    // batch 2: 1385 - 273 = 1112
    // batch 3: 1506 - 273 = 1233
    let batch = state
        .previous_batches
        .load(deps.as_ref().storage, 2u64.into())
        .unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(1345),
            uosmo_unclaimed: Uint128::new(1112), // 1385 - 273
            est_unbond_end_time: 20000,
        }
    );

    let batch = state
        .previous_batches
        .load(deps.as_ref().storage, 3u64.into())
        .unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 3,
            reconciled: true,
            total_shares: Uint128::new(1456),
            uosmo_unclaimed: Uint128::new(1233), // 1506 - 273
            est_unbond_end_time: 30000,
        }
    );

    // Batches 1 and 4 should not have changed
    let batch = state
        .previous_batches
        .load(deps.as_ref().storage, 1u64.into())
        .unwrap();
    assert_eq!(batch, previous_batches[0]);

    let batch = state
        .previous_batches
        .load(deps.as_ref().storage, 4u64.into())
        .unwrap();
    assert_eq!(batch, previous_batches[3]);
}

#[test]
fn withdrawing_unbonded() {
    let mut deps = setup_test();
    let state = State::default();

    // We simulate a most general case:
    // - batches 1 and 2 have finished unbonding
    // - batch 3 have been submitted for unbonding but have not finished
    // - batch 4 is still pending
    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420),
        },
        UnbondRequest {
            id: 2,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(34567),
        },
        UnbondRequest {
            id: 3,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(45678),
        },
        UnbondRequest {
            id: 4,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(56789),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (
                    unbond_request.id.into(),
                    &Addr::unchecked(unbond_request.user.clone()),
                ),
                unbond_request,
            )
            .unwrap();
    }

    let previous_batches = vec![
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(92876),
            uosmo_unclaimed: Uint128::new(95197), // 1.025 OSMO per Steak
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(34567),
            uosmo_unclaimed: Uint128::new(35604), // 1.030 OSMO per Steak
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 3,
            reconciled: false, // finished unbonding, but not reconciled; ignored
            total_shares: Uint128::new(45678),
            uosmo_unclaimed: Uint128::new(47276), // 1.035 OSMO per Steak
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 4,
            reconciled: true,
            total_shares: Uint128::new(56789),
            uosmo_unclaimed: Uint128::new(59060), // 1.040 OSMO per Steak
            est_unbond_end_time: 30000, // reconciled, but not yet finished unbonding; ignored
        },
    ];

    for previous_batch in &previous_batches {
        state
            .previous_batches
            .save(
                deps.as_mut().storage,
                previous_batch.id.into(),
                previous_batch,
            )
            .unwrap();
    }

    state
        .pending_batch
        .save(
            deps.as_mut().storage,
            &PendingBatch {
                id: 4,
                usteak_to_burn: Uint128::new(56789),
                est_unbond_start_time: 100000,
            },
        )
        .unwrap();

    // Attempt to withdraw before any batch has completed unbonding. Should error
    let err = execute(
        deps.as_mut(),
        mock_env_at_timestamp(5000),
        mock_info("user_1", &[]),
        ExecuteMsg::WithdrawUnbonded { receiver: None },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::ZeroWithdrawableAmount {});

    // Attempt to withdraw once batches 1 and 2 have finished unbonding, but 3 has not yet
    //
    // Withdrawable from batch 1: 95,197 * 23,456 / 92,876 = 24,042
    // Withdrawable from batch 2: 35,604
    // Total withdrawable: 24,042 + 35,604 = 59,646
    //
    // Batch 1 should be updated:
    // Total shares: 92,876 - 23,456 = 69,420
    // Unclaimed uosmo: 95,197 - 24,042 = 71,155
    //
    // Batch 2 is completely withdrawn, should be purged from storage
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(25000),
        mock_info("user_1", &[]),
        ExecuteMsg::WithdrawUnbonded { receiver: None },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "user_1".to_string(),
                amount: vec![Coin::new(59646, "uosmo")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // Previous batches should have been updated
    let batch = state
        .previous_batches
        .load(deps.as_ref().storage, 1u64.into())
        .unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(69420),
            uosmo_unclaimed: Uint128::new(71155),
            est_unbond_end_time: 10000,
        }
    );

    let err = state
        .previous_batches
        .load(deps.as_ref().storage, 2u64.into())
        .unwrap_err();
    assert_eq!(
        err,
        StdError::NotFound {
            kind: "steak::hub::Batch".to_string()
        }
    );

    // User 1's unbond requests in batches 1 and 2 should have been deleted
    let err1 = state
        .unbond_requests
        .load(
            deps.as_ref().storage,
            (1u64.into(), &Addr::unchecked("user_1")),
        )
        .unwrap_err();
    let err2 = state
        .unbond_requests
        .load(
            deps.as_ref().storage,
            (1u64.into(), &Addr::unchecked("user_1")),
        )
        .unwrap_err();

    assert_eq!(
        err1,
        StdError::NotFound {
            kind: "steak::hub::UnbondRequest".to_string()
        }
    );
    assert_eq!(
        err2,
        StdError::NotFound {
            kind: "steak::hub::UnbondRequest".to_string()
        }
    );

    // User 3 attempt to withdraw; also specifying a receiver
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(25000),
        mock_info("user_3", &[]),
        ExecuteMsg::WithdrawUnbonded {
            receiver: Some("user_2".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "user_2".to_string(),
                amount: vec![Coin::new(71155, "uosmo")]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // Batch 1 and user 2's unbonding request should have been purged from storage
    let err = state
        .previous_batches
        .load(deps.as_ref().storage, 1u64.into())
        .unwrap_err();
    assert_eq!(
        err,
        StdError::NotFound {
            kind: "steak::hub::Batch".to_string()
        }
    );

    let err = state
        .unbond_requests
        .load(
            deps.as_ref().storage,
            (1u64.into(), &Addr::unchecked("user_3")),
        )
        .unwrap_err();

    assert_eq!(
        err,
        StdError::NotFound {
            kind: "steak::hub::UnbondRequest".to_string()
        }
    );
}

#[test]
fn adding_validator() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::AddValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("apollo", &[]),
        ExecuteMsg::AddValidator {
            validator: "alice".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err("validator is already whitelisted"))
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("apollo", &[]),
        ExecuteMsg::AddValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let validators = state.validators.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        validators,
        vec![
            String::from("alice"),
            String::from("bob"),
            String::from("charlie"),
            String::from("dave")
        ],
    );
}

#[test]
fn removing_validator() {
    let mut deps = setup_test();
    let state = State::default();

    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "charlie".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("apollo", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err(
            "validator is not already whitelisted"
        ))
    );

    // Target: (341667 + 341667 + 341666) / 2 = 512500
    // Remainder: 0
    // Alice:   512500 + 0 - 341667 = 170833
    // Bob:     512500 + 0 - 341667 = 170833
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("apollo", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "charlie".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(
            Redelegation::new("charlie", "alice", 170833).to_cosmos_msg(),
            1
        ),
    );
    assert_eq!(
        res.messages[1],
        SubMsg::reply_on_success(
            Redelegation::new("charlie", "bob", 170833).to_cosmos_msg(),
            1
        ),
    );

    let validators = state.validators.load(deps.as_ref().storage).unwrap();
    assert_eq!(validators, vec![String::from("alice"), String::from("bob")],);
}

#[test]
fn transferring_ownership() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("apollo", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("apollo"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::AcceptOwnership {},
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::AcceptOwnership {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("jake"));
}

//--------------------------------------------------------------------------------------------------
// Queries
//--------------------------------------------------------------------------------------------------

#[test]
fn querying_previous_batches() {
    let mut deps = mock_dependencies();

    let batches = vec![
        Batch {
            id: 1,
            reconciled: false,
            total_shares: Uint128::new(123),
            uosmo_unclaimed: Uint128::new(678),
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(234),
            uosmo_unclaimed: Uint128::new(789),
            est_unbond_end_time: 15000,
        },
        Batch {
            id: 3,
            reconciled: false,
            total_shares: Uint128::new(345),
            uosmo_unclaimed: Uint128::new(890),
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 4,
            reconciled: true,
            total_shares: Uint128::new(456),
            uosmo_unclaimed: Uint128::new(999),
            est_unbond_end_time: 25000,
        },
    ];

    let state = State::default();
    for batch in &batches {
        state
            .previous_batches
            .save(deps.as_mut().storage, batch.id.into(), batch)
            .unwrap();
    }

    // Querying a single batch
    let res: Batch = query_helper(deps.as_ref(), QueryMsg::PreviousBatch(1));
    assert_eq!(res, batches[0].clone());

    let res: Batch = query_helper(deps.as_ref(), QueryMsg::PreviousBatch(2));
    assert_eq!(res, batches[1].clone());

    // Query multiple batches
    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, batches.clone());

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(1),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![batches[1].clone(), batches[2].clone(), batches[3].clone()]
    );

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(4),
            limit: None,
        },
    );
    assert_eq!(res, vec![]);

    // Query multiple batches, indexed by whether it has been reconciled
    let res = state
        .previous_batches
        .idx
        .reconciled
        .prefix(true.into())
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item.unwrap();
            v
        })
        .collect::<Vec<_>>();

    assert_eq!(res, vec![batches[1].clone(), batches[3].clone()]);

    let res = state
        .previous_batches
        .idx
        .reconciled
        .prefix(false.into())
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item.unwrap();
            v
        })
        .collect::<Vec<_>>();

    assert_eq!(res, vec![batches[0].clone(), batches[2].clone()]);
}

#[test]
fn querying_unbond_requests() {
    let mut deps = mock_dependencies();
    let state = State::default();

    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(123),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("bob"),
            shares: Uint128::new(234),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("charlie"),
            shares: Uint128::new(345),
        },
        UnbondRequest {
            id: 2,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(456),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (
                    unbond_request.id.into(),
                    &Addr::unchecked(unbond_request.user.clone()),
                ),
                unbond_request,
            )
            .unwrap();
    }

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 1,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            unbond_requests[0].clone().into(),
            unbond_requests[1].clone().into(),
            unbond_requests[2].clone().into(),
        ]
    );

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 2,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_requests[3].clone().into()]);

    let res: Vec<UnbondRequestsByUserResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUser {
            user: "alice".to_string(),
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            unbond_requests[0].clone().into(),
            unbond_requests[3].clone().into()
        ]
    );

    let res: Vec<UnbondRequestsByUserResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUser {
            user: "alice".to_string(),
            start_after: Some(2),
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_requests[3].clone().into()]);
}

//--------------------------------------------------------------------------------------------------
// Delegations
//--------------------------------------------------------------------------------------------------

#[test]
fn computing_undelegations() {
    let current_delegations = vec![
        Delegation::new("alice", 400),
        Delegation::new("bob", 300),
        Delegation::new("charlie", 200),
    ];

    // Target: (400 + 300 + 200 - 451) / 3 = 149
    // Remainder: 2
    // Alice:   400 - (149 + 1) = 250
    // Bob:     300 - (149 + 1) = 150
    // Charlie: 200 - (149 + 0) = 51
    let new_undelegations = compute_undelegations(Uint128::new(451), &current_delegations);
    let expected = vec![
        Undelegation::new("alice", 250),
        Undelegation::new("bob", 150),
        Undelegation::new("charlie", 51),
    ];
    assert_eq!(new_undelegations, expected);
}

#[test]
fn computing_redelegations_for_removal() {
    let current_delegations = vec![
        Delegation::new("alice", 13000),
        Delegation::new("bob", 12000),
        Delegation::new("charlie", 11000),
        Delegation::new("dave", 10000),
    ];

    // Suppose Dave will be removed
    // uosmo_per_validator = (13000 + 12000 + 11000 + 10000) / 3 = 15333
    // remainder = 1
    // to Alice:   15333 + 1 - 13000 = 2334
    // to Bob:     15333 + 0 - 12000 = 3333
    // to Charlie: 15333 + 0 - 11000 = 4333
    let expected = vec![
        Redelegation::new("dave", "alice", 2334),
        Redelegation::new("dave", "bob", 3333),
        Redelegation::new("dave", "charlie", 4333),
    ];

    assert_eq!(
        compute_redelegations_for_removal(&current_delegations[3], &current_delegations[..3]),
        expected,
    );
}

#[test]
fn computing_redelegations_for_rebalancing() {
    let current_delegations = vec![
        Delegation::new("alice", 69420),
        Delegation::new("bob", 1234),
        Delegation::new("charlie", 88888),
        Delegation::new("dave", 40471),
        Delegation::new("evan", 2345),
    ];

    // uosmo_per_validator = (69420 + 88888 + 1234 + 40471 + 2345) / 4 = 40471
    // remainer = 3
    // src_delegations:
    //  - alice:   69420 - (40471 + 1) = 28948
    //  - charlie: 88888 - (40471 + 1) = 48416
    // dst_delegations:
    //  - bob:     (40471 + 1) - 1234  = 39238
    //  - evan:    (40471 + 0) - 2345  = 38126
    //
    // Round 1: alice --(28948)--> bob
    // src_delegations:
    //  - charlie: 48416
    // dst_delegations:
    //  - bob:     39238 - 28948 = 10290
    //  - evan:    38126
    //
    // Round 2: charlie --(10290)--> bob
    // src_delegations:
    //  - charlie: 48416 - 10290 = 38126
    // dst_delegations:
    //  - evan:    38126
    //
    // Round 3: charlie --(38126)--> evan
    // Queues are emptied
    let expected = vec![
        Redelegation::new("alice", "bob", 28948),
        Redelegation::new("charlie", "bob", 10290),
        Redelegation::new("charlie", "evan", 38126),
    ];

    assert_eq!(
        compute_redelegations_for_rebalancing(&current_delegations),
        expected,
    );
}

//--------------------------------------------------------------------------------------------------
// Coins
//--------------------------------------------------------------------------------------------------

#[test]
fn parsing_coin() {
    let coin = parse_coin("12345uatom").unwrap();
    assert_eq!(coin, Coin::new(12345, "uatom"));

    let coin =
        parse_coin("23456ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B")
            .unwrap();
    assert_eq!(
        coin,
        Coin::new(
            23456,
            "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"
        )
    );

    let err = parse_coin("69420").unwrap_err();
    assert_eq!(err, StdError::generic_err("failed to parse coin: 69420"));

    let err = parse_coin("ngmi").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("Parsing u128: cannot parse integer from empty string")
    );
}

#[test]
fn parsing_coins() {
    let coins = Coins::from_str("").unwrap();
    assert_eq!(coins.0, vec![]);

    let coins = Coins::from_str("12345uatom").unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

    let coins = Coins::from_str("12345uatom,23456uosmo").unwrap();
    assert_eq!(
        coins.0,
        vec![Coin::new(12345, "uatom"), Coin::new(23456, "uosmo")]
    );
}

#[test]
fn adding_coins() {
    let mut coins = Coins(vec![]);

    coins.add(&Coin::new(12345, "uatom")).unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

    coins.add(&Coin::new(23456, "uosmo")).unwrap();
    assert_eq!(
        coins.0,
        vec![Coin::new(12345, "uatom"), Coin::new(23456, "uosmo")]
    );

    coins
        .add_many(&Coins::from_str("76543uatom,69420uusd").unwrap())
        .unwrap();
    assert_eq!(
        coins.0,
        vec![
            Coin::new(88888, "uatom"),
            Coin::new(23456, "uosmo"),
            Coin::new(69420, "uusd")
        ]
    );
}

#[test]
fn receiving_funds() {
    let err = parse_received_fund(&[], "uosmo").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("must deposit exactly one coin; received 0")
    );

    let err = parse_received_fund(
        &[Coin::new(12345, "uatom"), Coin::new(23456, "uosmo")],
        "uosmo",
    )
    .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("must deposit exactly one coin; received 2")
    );

    let err = parse_received_fund(&[Coin::new(12345, "uatom")], "uosmo").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("expected uosmo deposit, received uatom")
    );

    let err = parse_received_fund(&[Coin::new(0, "uosmo")], "uosmo").unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("deposit amount must be non-zero")
    );

    let amount = parse_received_fund(&[Coin::new(69420, "uosmo")], "uosmo").unwrap();
    assert_eq!(amount, Uint128::new(69420));
}
