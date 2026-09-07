ARG K3S_TAG=v1.35.5-k3s1

FROM rancher/k3s:${K3S_TAG} AS k3s

# The stock K3s image is Alpine-based. The NVIDIA Container Runtime is
# installed on an Ubuntu CUDA base so the inner k3s containerd can launch GPU
# workloads with the runtime supplied by the outer Docker engine.
FROM nvidia/cuda:13.2.0-base-ubuntu24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl gnupg \
    && install --directory --mode=0755 /usr/share/keyrings \
    && curl --fail --silent --show-error https://nvidia.github.io/libnvidia-container/gpgkey \
        | gpg --dearmor --output /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg \
    && curl --fail --silent --show-error \
        https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list \
        | sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#' \
        > /etc/apt/sources.list.d/nvidia-container-toolkit.list \
    && apt-get update \
    && apt-get install --yes --no-install-recommends nvidia-container-runtime nvidia-container-toolkit \
    && apt-get clean \
    && rm --recursive --force /var/lib/apt/lists/*

COPY --from=k3s /bin /opt/k3s-bin

RUN cp --archive /opt/k3s-bin/. /bin/ \
    && rm --recursive --force /opt/k3s-bin

RUN mkdir --parents /var/lib/rancher/k3s/agent/etc/containerd/config-v3.toml.d \
    && printf 'hosts: files dns\n' > /etc/nsswitch.conf \
    && chmod 1777 /tmp

COPY config-v3.toml.d/10-nvidia.toml \
    /var/lib/rancher/k3s/agent/etc/containerd/config-v3.toml.d/10-nvidia.toml

ENV PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/bin/aux"

VOLUME ["/var/lib/kubelet", "/var/lib/rancher/k3s", "/var/lib/cni", "/var/log"]

ENTRYPOINT ["/bin/k3s"]
CMD ["agent"]
