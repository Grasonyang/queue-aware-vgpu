The controller deployment is kept as small, reviewable YAML files in this
directory. Apply the complete set with:

```bash
scripts/deploy-controller.sh --cluster gx10 --build
```

The directory also contains the multi-stage controller Dockerfile. The
ServiceMonitor is optional and is applied only when the Prometheus Operator
CRD is present.
