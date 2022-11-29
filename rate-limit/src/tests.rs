//! Unit tests for the rate limit.
#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::traits::BadOrigin;

#[test]
fn update_rate_limit_rule_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			RateLimit::update_rate_limit_rule(
				RuntimeOrigin::signed(ALICE),
				0,
				DOT.encode(),
				Some(RateLimitRule::NotAllowed),
			),
			BadOrigin
		);

		assert_eq!(RateLimit::rate_limit_rules(0, DOT.encode()), None);
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			DOT.encode(),
			Some(RateLimitRule::NotAllowed),
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::RateLimitRuleUpdated {
			rate_limiter_id: 0,
			encoded_key: DOT.encode(),
			update: Some(RateLimitRule::NotAllowed),
		}));
		assert_eq!(
			RateLimit::rate_limit_rules(0, DOT.encode()),
			Some(RateLimitRule::NotAllowed)
		);

		assert_noop!(
			RateLimit::update_rate_limit_rule(
				RuntimeOrigin::root(),
				0,
				DOT.encode(),
				Some(RateLimitRule::PerBlocks {
					blocks_count: 0,
					quota: 1,
				}),
			),
			Error::<Runtime>::InvalidRateLimitRule
		);
		assert_noop!(
			RateLimit::update_rate_limit_rule(
				RuntimeOrigin::root(),
				0,
				DOT.encode(),
				Some(RateLimitRule::PerSeconds {
					secs_count: 1,
					quota: 0,
				}),
			),
			Error::<Runtime>::InvalidRateLimitRule
		);
		assert_noop!(
			RateLimit::update_rate_limit_rule(
				RuntimeOrigin::root(),
				0,
				DOT.encode(),
				Some(RateLimitRule::TokenBucket {
					blocks_count: 100,
					quota_increment: 1000,
					max_quota: 0,
				}),
			),
			Error::<Runtime>::InvalidRateLimitRule
		);

		// update will reset RateLimitQuota
		RateLimitQuota::<Runtime>::insert(0, DOT.encode(), (10, 100));
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (10, 100));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			DOT.encode(),
			Some(RateLimitRule::TokenBucket {
				blocks_count: 100,
				quota_increment: 1000,
				max_quota: 10000,
			}),
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::RateLimitRuleUpdated {
			rate_limiter_id: 0,
			encoded_key: DOT.encode(),
			update: Some(RateLimitRule::TokenBucket {
				blocks_count: 100,
				quota_increment: 1000,
				max_quota: 10000,
			}),
		}));
		assert_eq!(
			RateLimit::rate_limit_rules(0, DOT.encode()),
			Some(RateLimitRule::TokenBucket {
				blocks_count: 100,
				quota_increment: 1000,
				max_quota: 10000,
			})
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));
	});
}

#[test]
fn add_whitelist_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			RateLimit::add_whitelist(RuntimeOrigin::signed(ALICE), 0, KeyFilter::Match(ALICE.encode())),
			BadOrigin
		);

		assert_eq!(RateLimit::bypass_limit_whitelist(0), vec![]);
		assert_ok!(RateLimit::add_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(ALICE.encode())
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::WhitelistFilterAdded {
			rate_limiter_id: 0,
		}));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![KeyFilter::Match(ALICE.encode())]
		);

		// add already existed.
		assert_noop!(
			RateLimit::add_whitelist(RuntimeOrigin::root(), 0, KeyFilter::Match(ALICE.encode())),
			Error::<Runtime>::FilterExisted
		);

		assert_ok!(RateLimit::add_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(BOB.encode())
		));
		assert_ok!(RateLimit::add_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(CHARLIE.encode())
		));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![
				KeyFilter::Match(ALICE.encode()),
				KeyFilter::Match(BOB.encode()),
				KeyFilter::Match(CHARLIE.encode())
			]
		);

		// exceed filters limit
		assert_noop!(
			RateLimit::add_whitelist(RuntimeOrigin::root(), 0, KeyFilter::Match(DAVE.encode())),
			Error::<Runtime>::MaxFilterExceeded
		);
	});
}

#[test]
fn remove_whitelist_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(RateLimit::add_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(ALICE.encode())
		));
		assert_ok!(RateLimit::add_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(BOB.encode())
		));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![KeyFilter::Match(ALICE.encode()), KeyFilter::Match(BOB.encode())]
		);

		assert_noop!(
			RateLimit::remove_whitelist(RuntimeOrigin::signed(ALICE), 0, KeyFilter::StartsWith(ALICE.encode())),
			BadOrigin
		);

		assert_noop!(
			RateLimit::remove_whitelist(RuntimeOrigin::root(), 0, KeyFilter::StartsWith(ALICE.encode())),
			Error::<Runtime>::FilterExisted
		);

		assert_ok!(RateLimit::remove_whitelist(
			RuntimeOrigin::root(),
			0,
			KeyFilter::Match(ALICE.encode())
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::WhitelistFilterRemoved {
			rate_limiter_id: 0,
		}));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![KeyFilter::Match(BOB.encode())]
		);
	});
}

#[test]
fn reset_whitelist_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			RateLimit::reset_whitelist(
				RuntimeOrigin::signed(ALICE),
				0,
				vec![KeyFilter::StartsWith(ALICE.encode())],
			),
			BadOrigin
		);

		// exceed filters limit
		assert_noop!(
			RateLimit::reset_whitelist(
				RuntimeOrigin::root(),
				0,
				vec![
					KeyFilter::StartsWith(ALICE.encode()),
					KeyFilter::StartsWith(DAVE.encode()),
					KeyFilter::StartsWith(CHARLIE.encode()),
					KeyFilter::StartsWith(DAVE.encode())
				],
			),
			Error::<Runtime>::MaxFilterExceeded
		);

		assert_eq!(RateLimit::bypass_limit_whitelist(0), vec![]);
		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![KeyFilter::Match(ALICE.encode()), KeyFilter::Match(BOB.encode())]
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::WhitelistFilterReset {
			rate_limiter_id: 0,
		}));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![KeyFilter::Match(ALICE.encode()), KeyFilter::Match(BOB.encode())]
		);

		// will sort KeyFilter list before insert.
		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![
				KeyFilter::Match(CHARLIE.encode()),
				KeyFilter::Match(BOB.encode()),
				KeyFilter::Match(ALICE.encode())
			]
		));
		System::assert_last_event(RuntimeEvent::RateLimit(crate::Event::WhitelistFilterReset {
			rate_limiter_id: 0,
		}));
		assert_eq!(
			RateLimit::bypass_limit_whitelist(0),
			vec![
				KeyFilter::Match(ALICE.encode()),
				KeyFilter::Match(BOB.encode()),
				KeyFilter::Match(CHARLIE.encode())
			]
		);

		// clear
		assert_ok!(RateLimit::reset_whitelist(RuntimeOrigin::root(), 0, vec![]));
		assert_eq!(RateLimit::bypass_limit_whitelist(0), vec![]);
	});
}

#[test]
fn bypass_limit_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(RateLimit::bypass_limit(0, BOB), false);
		assert_eq!(RateLimit::bypass_limit(1, BOB), false);
		assert_eq!(RateLimit::bypass_limit(0, TREASURY_ACCOUNT), false);

		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![KeyFilter::Match(BOB.encode())]
		));
		assert_eq!(RateLimit::bypass_limit(0, BOB), true);
		assert_eq!(RateLimit::bypass_limit(1, BOB), false);
		assert_eq!(RateLimit::bypass_limit(0, TREASURY_ACCOUNT), false);

		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![KeyFilter::StartsWith(vec![1, 1, 1, 1])]
		));
		assert_eq!(RateLimit::bypass_limit(0, BOB), true);
		assert_eq!(RateLimit::bypass_limit(0, TREASURY_ACCOUNT), true);

		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![KeyFilter::StartsWith(vec![1, 1, 1, 1, 1])]
		));
		assert_eq!(RateLimit::bypass_limit(0, BOB), true);
		assert_eq!(RateLimit::bypass_limit(0, TREASURY_ACCOUNT), false);
		assert_eq!(RateLimit::bypass_limit(0, CHARLIE), false);

		assert_ok!(RateLimit::reset_whitelist(
			RuntimeOrigin::root(),
			0,
			vec![
				KeyFilter::StartsWith(vec![1, 1, 1, 1, 1]),
				KeyFilter::EndsWith(vec![2, 2, 2, 2])
			]
		));
		assert_eq!(RateLimit::bypass_limit(0, BOB), true);
		assert_eq!(RateLimit::bypass_limit(0, TREASURY_ACCOUNT), true);
		assert_eq!(RateLimit::bypass_limit(0, CHARLIE), true);
	});
}

#[test]
fn access_remainer_quota_after_update_per_blocks() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(100);
		assert_eq!(System::block_number(), 100);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		// current - last_updated >= blocks_count, will update RateLimitQuota firstly
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerBlocks {
					blocks_count: 30,
					quota: 500,
				},
				&0,
				&DOT.encode(),
			),
			500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 500));

		// mock consume remainer_quota
		RateLimitQuota::<Runtime>::mutate(0, DOT.encode(), |(_, remainer_quota)| *remainer_quota = 400);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		// current - last_updated < blocks_count, will not update RateLimitQuota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerBlocks {
					blocks_count: 30,
					quota: 5000,
				},
				&0,
				&DOT.encode(),
			),
			400
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		System::set_block_number(119);
		assert_eq!(System::block_number(), 119);

		// current - last_updated < blocks_count, will not update RateLimitQuota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerBlocks {
					blocks_count: 20,
					quota: 100,
				},
				&0,
				&DOT.encode(),
			),
			400
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		System::set_block_number(120);
		assert_eq!(System::block_number(), 120);

		// current - last_updated > blocks_count, will reset remainer_quota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerBlocks {
					blocks_count: 20,
					quota: 100,
				},
				&0,
				&DOT.encode(),
			),
			100
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (120, 100));
	});
}

#[test]
fn access_remainer_quota_after_update_per_seconds() {
	ExtBuilder::default().build().execute_with(|| {
		Timestamp::set_timestamp(100_000);
		assert_eq!(<Timestamp as UnixTime>::now().as_secs(), 100);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		// current - last_updated >= secs_count, will update RateLimitQuota firstly
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerSeconds {
					secs_count: 30,
					quota: 500,
				},
				&0,
				&DOT.encode(),
			),
			500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 500));

		// mock consume remainer_quota
		RateLimitQuota::<Runtime>::mutate(0, DOT.encode(), |(_, remainer_quota)| *remainer_quota = 400);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		// current - last_updated < secs_count, will not update RateLimitQuota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerSeconds {
					secs_count: 30,
					quota: 5000,
				},
				&0,
				&DOT.encode(),
			),
			400
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		Timestamp::set_timestamp(119_000);
		assert_eq!(<Timestamp as UnixTime>::now().as_secs(), 119);

		// current - last_updated < secs_count, will not update RateLimitQuota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerSeconds {
					secs_count: 20,
					quota: 100,
				},
				&0,
				&DOT.encode(),
			),
			400
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 400));

		Timestamp::set_timestamp(120_000);
		assert_eq!(<Timestamp as UnixTime>::now().as_secs(), 120);

		// current - last_updated > secs_count, will reset remainer_quota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::PerSeconds {
					secs_count: 20,
					quota: 100,
				},
				&0,
				&DOT.encode(),
			),
			100
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (120, 100));
	});
}

#[test]
fn access_remainer_quota_after_update_token_bucket() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(100);
		assert_eq!(System::block_number(), 100);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		// (current - last_updated) / blocks_count = 3, will inc 3 * quota_increment
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::TokenBucket {
					blocks_count: 30,
					quota_increment: 500,
					max_quota: 1500,
				},
				&0,
				&DOT.encode(),
			),
			1500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 1500));

		// mock consume remainer_quota
		RateLimitQuota::<Runtime>::mutate(0, DOT.encode(), |(_, remainer_quota)| *remainer_quota = 1400);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 1400));

		System::set_block_number(119);
		assert_eq!(System::block_number(), 119);

		// (current - last_updated) / blocks_count = 0, will not update RateLimitQuota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::TokenBucket {
					blocks_count: 30,
					quota_increment: 500,
					max_quota: 1500,
				},
				&0,
				&DOT.encode(),
			),
			1400
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 1400));

		System::set_block_number(130);
		assert_eq!(System::block_number(), 130);

		// (current - last_updated) / blocks_count = 1, will inc quota_increment, but
		// remainer_quota always <= max_quota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::TokenBucket {
					blocks_count: 30,
					quota_increment: 500,
					max_quota: 1500,
				},
				&0,
				&DOT.encode(),
			),
			1500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (130, 1500));

		System::set_block_number(160);
		assert_eq!(System::block_number(), 160);

		// (current - last_updated) / blocks_count = 1, will inc quota_increment, but
		// remainer_quota always <= max_quota
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(
				RateLimitRule::TokenBucket {
					blocks_count: 30,
					quota_increment: 500,
					max_quota: 200,
				},
				&0,
				&DOT.encode(),
			),
			200
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (160, 200));
	});
}

#[test]
fn access_remainer_quota_after_update_when_not_allowed_or_unlimited() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(100);
		assert_eq!(System::block_number(), 100);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		assert_eq!(
			RateLimit::access_remainer_quota_after_update(RateLimitRule::NotAllowed, &0, &DOT.encode(),),
			0
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(RateLimitRule::Unlimited, &0, &DOT.encode(),),
			0
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		RateLimitQuota::<Runtime>::mutate(0, DOT.encode(), |(_, remainer_quota)| *remainer_quota = 500);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 500));

		assert_eq!(
			RateLimit::access_remainer_quota_after_update(RateLimitRule::NotAllowed, &0, &DOT.encode(),),
			500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 500));
		assert_eq!(
			RateLimit::access_remainer_quota_after_update(RateLimitRule::Unlimited, &0, &DOT.encode(),),
			500
		);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 500));
	});
}

#[test]
fn record_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			DOT.encode(),
			Some(RateLimitRule::PerBlocks {
				blocks_count: 30,
				quota: 500,
			}),
		));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			BTC.encode(),
			Some(RateLimitRule::Unlimited),
		));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			1,
			ETH.encode(),
			Some(RateLimitRule::PerBlocks {
				blocks_count: 30,
				quota: 500,
			}),
		));

		RateLimitQuota::<Runtime>::mutate(0, DOT.encode(), |(_, remainer_quota)| *remainer_quota = 10000);
		RateLimitQuota::<Runtime>::mutate(0, BTC.encode(), |(_, remainer_quota)| *remainer_quota = 100);
		RateLimitQuota::<Runtime>::mutate(0, ETH.encode(), |(_, remainer_quota)| *remainer_quota = 1000);
		RateLimitQuota::<Runtime>::mutate(1, ETH.encode(), |(_, remainer_quota)| *remainer_quota = 1000);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 10000));
		assert_eq!(RateLimit::rate_limit_quota(0, BTC.encode()), (0, 100));
		assert_eq!(RateLimit::rate_limit_quota(0, ETH.encode()), (0, 1000));
		assert_eq!(RateLimit::rate_limit_quota(1, ETH.encode()), (0, 1000));

		// will consume
		RateLimit::record(0, DOT, 1000);
		RateLimit::record(1, ETH, 500);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 9000));
		assert_eq!(RateLimit::rate_limit_quota(1, ETH.encode()), (0, 500));

		// will not consume
		RateLimit::record(0, BTC, 100);
		RateLimit::record(0, ETH, 500);
		assert_eq!(RateLimit::rate_limit_quota(0, BTC.encode()), (0, 100));
		assert_eq!(RateLimit::rate_limit_quota(0, ETH.encode()), (0, 1000));

		// consume when vaule > remainer_quota
		RateLimit::record(1, ETH, 1000);
		assert_eq!(RateLimit::rate_limit_quota(1, ETH.encode()), (0, 0));
	});
}

#[test]
fn is_allowed_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			DOT.encode(),
			Some(RateLimitRule::PerBlocks {
				blocks_count: 30,
				quota: 500,
			}),
		));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			1,
			DOT.encode(),
			Some(RateLimitRule::TokenBucket {
				blocks_count: 30,
				quota_increment: 500,
				max_quota: 1000,
			}),
		));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			0,
			BTC.encode(),
			Some(RateLimitRule::NotAllowed),
		));
		assert_ok!(RateLimit::update_rate_limit_rule(
			RuntimeOrigin::root(),
			1,
			BTC.encode(),
			Some(RateLimitRule::Unlimited),
		));

		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));
		assert_ok!(RateLimit::is_allowed(0, DOT, 0));
		assert_eq!(RateLimit::is_allowed(0, DOT, 500), Err(RateLimiterError::ExceedLimit),);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));

		assert_eq!(RateLimit::rate_limit_quota(1, DOT.encode()), (0, 0));
		assert_ok!(RateLimit::is_allowed(1, DOT, 0));
		assert_eq!(RateLimit::is_allowed(1, DOT, 501), Err(RateLimiterError::ExceedLimit),);
		assert_eq!(RateLimit::rate_limit_quota(1, DOT.encode()), (0, 0));

		System::set_block_number(100);
		assert_eq!(System::block_number(), 100);

		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (0, 0));
		assert_ok!(RateLimit::is_allowed(0, DOT, 500));
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 500));
		assert_eq!(RateLimit::is_allowed(0, DOT, 501), Err(RateLimiterError::ExceedLimit),);
		assert_eq!(RateLimit::rate_limit_quota(0, DOT.encode()), (100, 500));

		assert_eq!(RateLimit::rate_limit_quota(1, DOT.encode()), (0, 0));
		assert_ok!(RateLimit::is_allowed(1, DOT, 501));
		assert_eq!(RateLimit::rate_limit_quota(1, DOT.encode()), (100, 1000));
		assert_eq!(RateLimit::is_allowed(1, DOT, 1001), Err(RateLimiterError::ExceedLimit),);
		assert_eq!(RateLimit::rate_limit_quota(1, DOT.encode()), (100, 1000));

		// NotAllowed always return error, even if value is 0
		RateLimitQuota::<Runtime>::mutate(0, BTC.encode(), |(_, remainer_quota)| *remainer_quota = 10000);
		assert_eq!(RateLimit::rate_limit_quota(0, BTC.encode()), (0, 10000));
		assert_eq!(RateLimit::is_allowed(0, BTC, 0), Err(RateLimiterError::ExceedLimit),);
		assert_eq!(RateLimit::is_allowed(0, BTC, 100), Err(RateLimiterError::ExceedLimit),);

		// Unlimited always return true
		assert_eq!(RateLimit::rate_limit_quota(1, BTC.encode()), (0, 0));
		assert_ok!(RateLimit::is_allowed(1, BTC, 0));
		assert_ok!(RateLimit::is_allowed(1, BTC, 10000));
		assert_ok!(RateLimit::is_allowed(1, BTC, u128::MAX));

		// if dosen't config rule, always return true
		assert_eq!(RateLimitRules::<Runtime>::contains_key(0, ETH.encode()), false);
		assert_ok!(RateLimit::is_allowed(0, ETH, 10000));
		assert_ok!(RateLimit::is_allowed(0, ETH, u128::MAX));
	});
}
