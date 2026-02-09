
// This function is used to get the number of shards based on the available parallelism.
// It calculates the optimal number of shards based on the square of the parallelism divided by 4.
// It then finds the closest power of 2 to that number and returns it.
pub fn get_number_of_shards(parallelism: usize) -> usize {
    let parallelism = parallelism.max(2);
    let parallelism = parallelism.min(192);

    let optimal_number_shards = parallelism.pow(2) / 4;
    if optimal_number_shards < 2 {
        return 2;
    }

    let closest_power_of_2 = optimal_number_shards.ilog2();
    let shards_power_of_2 = 2usize.pow(closest_power_of_2);
    info!("Available parallelism: {}", parallelism);
    info!("Optimal number of shards: {}", optimal_number_shards);
    info!("Closest power of 2: {}", closest_power_of_2);

    if shards_power_of_2 > 1 {
        shards_power_of_2
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_power_of_two(x: usize) -> bool {
        x != 0 && (x & (x - 1)) == 0
    }

    #[test]
    fn test_get_parallelism_returns_power_of_two() {
        for parallelism in vec![
            3,
            7,
            11,
            15,
            21,
            31,
            63,
            127,
            4096,
            8192,
            9_223_372_036_854_775_783,
            usize::MAX / 2,
            usize::MAX,
        ] {
            let shards = get_number_of_shards(parallelism);
            assert!(
                is_power_of_two(shards),
                "Returned value {} is not a power of 2 for parallelism {}",
                shards,
                parallelism
            );
        }
    }

    #[test]
    fn test_get_parallelism_minimum_value() {
        // Should never return less than 2
        assert_eq!(get_number_of_shards(0), 2);
        assert_eq!(get_number_of_shards(1), 2);
        assert_eq!(get_number_of_shards(2), 2);
    }
}
