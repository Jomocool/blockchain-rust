pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, Weight},
	};

	parameter_types! {
				pub const ExtrinsicBaseWeight: Weight =
			Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(125_000), 0);
	}

	#[cfg(test)]
	mod test_weights {
		use frame_support::weights::constants;

		#[test]
		fn sane() {
			let w = super::constants::ExtrinsicBaseWeight::get();

			assert!(
				w.ref_time() >= 10u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
				"Weight should be at least 10 µs."
			);
			assert!(
				w.ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Weight should be at most 1 ms."
			);
		}
	}
}
