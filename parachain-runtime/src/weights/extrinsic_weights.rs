pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, Weight},
	};

	// 定义参数类型，用于指定交易的基本权重
	parameter_types! {
		// ExtrinsicBaseWeight 表示交易基础权重
		pub const ExtrinsicBaseWeight: Weight =
			// 基础权重计算方式：将每纳秒的权重参考时间乘以125000纳秒
			Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(125_000), 0);
	}

	// 测试模块，用于验证权重参数的合理性
	#[cfg(test)]
	mod test_weights {
		use frame_support::weights::constants;

		// 测试函数 sane 用于检查 ExtrinsicBaseWeight 的值是否在合理范围内
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
