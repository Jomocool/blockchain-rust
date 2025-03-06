pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, RuntimeDbWeight},
	};

	// 定义数据库操作的权重参数
	parameter_types! {
		// 定义RocksDbWeight常量，用于表示数据库读写操作的权重
		pub const RocksDbWeight: RuntimeDbWeight = RuntimeDbWeight {
			// 数据库读操作的权重，单位是纳秒
			read: 25_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
			// 数据库写操作的权重，单位是纳秒
			write: 100_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
		};
	}

	#[cfg(test)]
	mod test_db_weights {
		use super::constants::RocksDbWeight as W;
		use frame_support::weights::constants;

		#[test]
		fn sane() {
			assert!(
				W::get().reads(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Read weight should be at least 1 µs."
			);
			assert!(
				W::get().writes(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Write weight should be at least 1 µs."
			);
			assert!(
				W::get().reads(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Read weight should be at most 1 ms."
			);
			assert!(
				W::get().writes(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Write weight should be at most 1 ms."
			);
		}
	}
}
