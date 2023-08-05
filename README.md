
# configurr

A simple, lightweight project for when you want to violate one of the core principles of k8s.

What? ConfigMaps are supposed to be read only from the data plane? Nonsense.

This container will check a file for changes, then push them to a ConfigMap. It's designed to be run as a sidecar container in an existing pod.

## Motivation

I really wanted [wireguard-ui](https://github.com/ngoduykhanh/wireguard-ui/tree/master) to be able to configure identical wireguard instances deployed for my entire cluster. I used [Reloader](https://github.com/stakater/Reloader) to propagate configmap changes to wireguard pod restarts, and then created this project to forward a PVC file change to the configmap.

## Example Deployment

Check out the `examples` dir for a deployment example.