To seperate concenrs Tenants have seperate overlay networks their zones and VM's are attached to.

Two node Overlay with Direct forwarding plugin
Node03 (NAS)
```bash
dladm create-overlay \
		-v 0 \
		-e vxlan \
		-s direct \
		-p vxlan/listen_ip=192.168.1.81 \
		-p vxlan/listen_port=4789 \
		-p direct/dest_ip=192.168.1.80 \
		-p direct/dest_port=4789 \
		testnet0
```

DevNode
```bash
dladm create-overlay \
        -v 0 \
        -e vxlan \
        -s direct \
        -p vxlan/listen_ip=192.168.1.80 \
        -p vxlan/listen_port=4789 \
        -p direct/dest_ip=192.168.1.81 \
        -p direct/dest_port=4789 \
        testnet0
```