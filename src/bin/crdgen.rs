use kube::CustomResourceExt;
use queue_aware_vgpu_controller::crd::VGPUQueue;

fn main() {
    println!(
        "{}",
        serde_yaml::to_string(&VGPUQueue::crd()).expect("serialize CRD")
    );
}
