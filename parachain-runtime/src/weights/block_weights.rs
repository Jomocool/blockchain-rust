pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, Weight},
	};

	parameter_types! {
				pub const BlockExecutionWeight: Weight =
			Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(5_000_000), 0);
	}

	#[cfg(test)]
	mod test_weights {
		use frame_support::weights::constants;

		#[test]
		fn sane() {
			let w = super::constants::BlockExecutionWeight::get();

			assert!(
				w.ref_time() >= 100u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
				"Weight should be at least 100 µs."
			);
			assert!(
				w.ref_time() <= 50u64 * constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Weight should be at most 50 ms."
			);
		}
	}
}
