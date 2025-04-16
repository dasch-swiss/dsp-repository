use services::calculator::CalculatorServiceImpl;
use services::metadata::MetadataServiceImpl;
use storage::metadata::InMemoryMetadataRepository;

#[derive(Debug, Clone)]
pub struct AppState {
    pub metadata_service: MetadataServiceImpl<InMemoryMetadataRepository>,
    pub calculator_service: CalculatorServiceImpl,
}