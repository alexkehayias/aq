use lazy_static;


type AirQualityRow = (f32, f32, i32, i32, &'static str);

lazy_static! {
    /// A lookup table of concentration high, low, index high, low, and
    /// label based on EPA guidance for PM2.5 pollution
    static ref PM2_5_LOOKUP_TABLE: Vec<AirQualityRow> = vec![
        (0.0,   12.0,  0,   50,  "good"),
        (12.1,  35.4,  51,  100, "moderate"),
        (35.5,  55.4,  101, 150, "unhealthy for sensitive groups"),
        (55.5,  150.4, 151, 200, "unhealthy"),
        (150.5, 250.4, 201, 300, "very unhealthy"),
        (250.5, 350.4, 301, 400, "hazardous"),
        (350.5, 500.4, 401, 500, "hazardous"),
    ];
}

/// Finds the breakpoints using the air quality table
fn find_lookup_values(concentration: f32) -> AirQualityRow {
    if concentration > 500.4 {
        return *PM2_5_LOOKUP_TABLE.last().unwrap()
    }

    // TODO is there a nicer way to do this without an intermediate
    // Option type?
    let mut row = None;
    for r in PM2_5_LOOKUP_TABLE.iter() {
        let (low, high, _, _, _) = r;
        if concentration >= *low && concentration <= *high {
            row = Some(*r);
            break
        }
    }

    row.unwrap()
}

fn aqi(lookup_values: AirQualityRow, concentration: f32) -> f32 {
    let (c_low, c_high, i_low, i_high, _) = lookup_values;
    ((i_high - i_low) as f32 / (c_high - c_low))
        * (concentration - c_low)
        + i_low as f32
}

pub fn aqi_from_pm2_5(concentration: f32) -> f32 {
    let lookup_values = find_lookup_values(concentration);
    aqi(lookup_values, concentration)
}

#[cfg(test)]
mod test_aqi {
    use super::*;

    #[test]
    fn test_aqi() {
        let result = aqi_from_pm2_5(12.0);
        assert_eq!(50.0, result);

        let result = aqi_from_pm2_5(0.0);
        assert_eq!(0.0, result);
    }
}
