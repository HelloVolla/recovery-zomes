#[cfg(test)]
mod dna_path;

#[cfg(all(test, feature = "single_agent"))]
mod single_agent;

#[cfg(all(test, feature = "multi_agent"))]
mod multi_agent;

#[cfg(all(test, feature = "registry_test"))]
mod registry;
