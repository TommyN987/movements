fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../publisher_service/proto/movements.proto")?;
    Ok(())
}
