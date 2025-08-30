// Integration tests for rustbox

use crate::tests::common::TestConfig;
use anyhow::Result;

#[cfg(test)]
pub fn test_core_functionality() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::core::run_core_tests(&config)?;
    assert!(!results.is_empty());
    println!(
        "Core tests completed: {}/{} passed",
        results.iter().filter(|r| r.passed).count(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
pub fn test_resource_management() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::resource::run_resource_tests(&config)?;
    assert!(!results.is_empty());
    println!(
        "Resource tests completed: {}/{} passed",
        results.iter().filter(|r| r.passed).count(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
pub fn test_security_features() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::security::run_security_tests(&config)?;
    assert!(!results.is_empty());
    println!(
        "Security tests completed: {}/{} passed",
        results.iter().filter(|r| r.passed).count(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
pub fn test_stress_scenarios() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::stress::run_stress_tests(&config)?;
    assert!(!results.is_empty());
    println!(
        "Stress tests completed: {}/{} passed",
        results.iter().filter(|r| r.passed).count(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
pub fn test_performance_benchmarks() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::performance::run_performance_tests(&config)?;
    assert!(!results.is_empty());
    println!(
        "Performance tests completed: {}/{} passed",
        results.iter().filter(|r| r.passed).count(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
pub fn test_language_support() -> Result<()> {
    let config = TestConfig::default();
    let results = crate::tests::languages::run_language_tests(&config)?;
    assert!(!results.is_empty());
    
    let passed_count = results.iter().filter(|r| r.passed).count();
    println!(
        "Language tests completed: {}/{} passed",
        passed_count,
        results.len()
    );
    
    // Print details of failed tests
    for result in &results {
        if !result.passed {
            println!("FAILED: {} - {}", result.name, result.error_message.as_ref().unwrap_or(&"No error message".to_string()));
        } else {
            println!("PASSED: {}", result.name);
        }
    }
    Ok(())
}