#!/bin/sh
# netifd protocol handler for Vor — registers the 'vor' pseudo-protocol so
# router users can manage the relay through the standard network config:
#
#   config interface 'vor0'
#       option proto 'vor'
#       option listen_port '40443'
#       option edge_ip '104.18.1.1'
#       option edge_port '443'
#       option strategy 'sni_split'

[ -n "$INCLUDE_ONLY" ] || {
	. /lib/functions.sh
	. /lib/netifd/netifd-proto.sh
	init_proto "$@"
}

proto_vor_init_config() {
	no_device=1
	available=1
	proto_config_add_string "listen_port"
	proto_config_add_string "edge_ip"
	proto_config_add_string "edge_port"
	proto_config_add_string "strategy"
}

proto_vor_setup() {
	local config="$1"
	local iface="$2"

	local listen_port edge_ip edge_port strategy
	json_get_vars listen_port edge_ip edge_port strategy

	# Persist the interface options into the daemon's UCI config and
	# (re)start via procd.
	uci_set() {
		local key="$1" value="$2"
		[ -n "$value" ] || return 0
		uci -q set "vor.vor.$key=$value" 2>/dev/null || {
			touch /etc/config/vor
			printf "config vor 'vor'\\n" > /etc/config/vor
		}
	}
	uci_set listen_port "$listen_port"
	uci_set edge_ip "$edge_ip"
	uci_set edge_port "$edge_port"
	uci_set strategy "$strategy"
	uci -q commit vor 2>/dev/null

	/etc/init.d/vor restart
	proto_init_update "$iface" 1
	proto_send_update "$config"
}

proto_vor_teardown() {
	local config="$1"
	/etc/init.d/vor stop
	proto_init_update "*" 0
	proto_send_update "$config"
}

[ -n "$INCLUDE_ONLY" ] || {
	add_protocol vor
}
