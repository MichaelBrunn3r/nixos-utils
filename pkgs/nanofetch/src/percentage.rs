#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PercentageLevel {
    Normal,
    Warning,
    Critical,
}

impl PercentageLevel {
    pub fn from_percentage(percentage: f64) -> Self {
        if percentage >= 90.0 {
            Self::Critical
        } else if percentage >= 75.0 {
            Self::Warning
        } else {
            Self::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PercentageLevel;

    #[test]
    fn classifies_percentage_thresholds() {
        assert_eq!(
            PercentageLevel::from_percentage(74.9),
            PercentageLevel::Normal
        );
        assert_eq!(
            PercentageLevel::from_percentage(75.0),
            PercentageLevel::Warning
        );
        assert_eq!(
            PercentageLevel::from_percentage(89.9),
            PercentageLevel::Warning
        );
        assert_eq!(
            PercentageLevel::from_percentage(90.0),
            PercentageLevel::Critical
        );
    }
}
