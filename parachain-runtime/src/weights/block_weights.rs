pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, Weight},
	};

	// 定义参数类型，用于配置块执行的权重
	parameter_types! {
		// BlockExecutionWeight 表示块执行的权重
		pub const BlockExecutionWeight: Weight =
			// 权重基于常量 WEIGHT_REF_TIME_PER_NANOS 乘以 5,000,000 计算得出
			Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(5_000_000), 0);
	}

	// 测试配置权重的合理性
	#[cfg(test)]
	mod test_weights {
		use frame_support::weights::constants;

		// 检查 BlockExecutionWeight 是否在合理的范围内
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
