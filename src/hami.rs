use anyhow::{Result, anyhow};
use k8s_openapi::api::core::v1::{Node, Pod};
use serde::Deserialize;

pub const GPU_RESOURCE: &str = "nvidia.com/gpu";
pub const MEMORY_RESOURCE: &str = "nvidia.com/gpumem";
pub const CORE_RESOURCE: &str = "nvidia.com/gpucores";
pub const NODE_REGISTER_ANNOTATION: &str = "hami.io/node-nvidia-register";
pub const ALLOCATED_ANNOTATION: &str = "hami.io/vgpu-devices-allocated";

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct HamiGpu {
    pub id: String,
    pub count: u32,
    pub devmem: u64,
    pub devcore: u32,
    #[serde(rename = "type")]
    pub gpu_type: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default = "default_true")]
    pub health: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuCapacity {
    pub replicas: u64,
    pub memory_mib: u64,
}

pub fn registered_gpus(node: &Node) -> Vec<HamiGpu> {
    let Some(value) = node
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(NODE_REGISTER_ANNOTATION))
    else {
        return Vec::new();
    };
    parse_registered_gpus(value).unwrap_or_default()
}

pub fn parse_registered_gpus(value: &str) -> Result<Vec<HamiGpu>> {
    if value.trim_start().starts_with('[') {
        return serde_json::from_str(value).map_err(Into::into);
    }

    let mut gpus = Vec::new();
    for (index, item) in value
        .split(';')
        .filter(|item| !item.trim().is_empty())
        .enumerate()
    {
        let fields: Vec<_> = item.split(',').map(str::trim).collect();
        if fields.len() < 4 {
            return Err(anyhow!("invalid legacy HAMi GPU registration: {item}"));
        }
        gpus.push(HamiGpu {
            id: fields[0].to_string(),
            gpu_type: fields.get(1).copied().unwrap_or("NVIDIA").to_string(),
            devmem: fields
                .get(2)
                .and_then(|v| v.parse().ok())
                .unwrap_or_default(),
            devcore: fields.get(3).and_then(|v| v.parse().ok()).unwrap_or(100),
            count: fields.get(4).and_then(|v| v.parse().ok()).unwrap_or(1),
            mode: format!("legacy-{index}"),
            health: true,
        });
    }
    Ok(gpus)
}

pub fn node_capacity(node: &Node) -> GpuCapacity {
    let gpus = registered_gpus(node);
    if !gpus.is_empty() {
        return gpus.into_iter().filter(|gpu| gpu.health).fold(
            GpuCapacity::default(),
            |mut capacity, gpu| {
                capacity.replicas += u64::from(gpu.count);
                capacity.memory_mib += gpu.devmem.saturating_mul(u64::from(gpu.count));
                capacity
            },
        );
    }

    let replicas = node_quantity(node, GPU_RESOURCE).unwrap_or_default();
    let memory_mib = node_quantity(node, MEMORY_RESOURCE).unwrap_or_default();
    GpuCapacity {
        replicas,
        memory_mib,
    }
}

pub fn node_quantity(node: &Node, resource: &str) -> Option<u64> {
    node.status
        .as_ref()?
        .allocatable
        .as_ref()?
        .get(resource)
        .and_then(|quantity| parse_quantity_mib(&quantity.0).ok())
}

pub fn pod_gpu_request(pod: &Pod) -> u64 {
    pod_resource_quantity(pod, GPU_RESOURCE).unwrap_or_default()
}

pub fn pod_memory_request_mib(pod: &Pod, default_mib: u64) -> u64 {
    pod_resource_quantity(pod, MEMORY_RESOURCE)
        .or_else(|| pod_resource_quantity(pod, "nvidia.com/gpu-memory"))
        .unwrap_or(default_mib)
}

pub fn pod_resource_quantity(pod: &Pod, resource: &str) -> Option<u64> {
    let spec = pod.spec.as_ref()?;
    spec.containers
        .iter()
        .filter_map(|container| container.resources.as_ref())
        .flat_map(|resources| {
            resources
                .limits
                .as_ref()
                .into_iter()
                .chain(resources.requests.as_ref())
        })
        .find_map(|resources| resources.get(resource))
        .and_then(|quantity| parse_quantity_mib(&quantity.0).ok())
}

pub fn allocated_memory_mib(pod: &Pod) -> u64 {
    pod.metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(ALLOCATED_ANNOTATION))
        .map(|value| parse_allocated_annotation(value))
        .unwrap_or_default()
}

pub fn parse_allocated_annotation(value: &str) -> u64 {
    value
        .split(';')
        .filter_map(|device| device.split(',').nth(2))
        .filter_map(|memory| memory.trim().parse::<u64>().ok())
        .sum()
}

pub fn parse_quantity_mib(value: &str) -> Result<u64> {
    let value = value.trim();
    let (number, multiplier) = if let Some(value) = value.strip_suffix("Gi") {
        (value, 1024_u64)
    } else if let Some(value) = value.strip_suffix("G") {
        (value, 1000_u64)
    } else if let Some(value) = value.strip_suffix("Mi") {
        (value, 1_u64)
    } else if let Some(value) = value.strip_suffix("M") {
        (value, 1_u64)
    } else if let Some(value) = value.strip_suffix("Ki") {
        let kib: u64 = value.parse()?;
        return Ok(kib / 1024);
    } else {
        (value, 1_u64)
    };
    Ok(number.parse::<u64>()?.saturating_mul(multiplier))
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gb10_register_fixture() {
        let value = include_str!("../tests/fixtures/hami-node-register.json");
        let gpus = parse_registered_gpus(value).expect("registration");
        assert_eq!(gpus[0].devmem, 131072);
        assert_eq!(gpus[0].count, 10);
    }

    #[test]
    fn parses_allocated_device_annotation() {
        assert_eq!(
            parse_allocated_annotation("GPU-1,NVIDIA,8192,20:;GPU-2,NVIDIA,4096,10:;"),
            12288
        );
    }

    #[test]
    fn parses_common_memory_quantities() {
        assert_eq!(parse_quantity_mib("8Gi").unwrap(), 8192);
        assert_eq!(parse_quantity_mib("512Mi").unwrap(), 512);
    }
}
