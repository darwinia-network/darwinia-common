#!/bin/sh

# $1 is uri
# $2, $3 is sr25519 private, public key
# $2, $3 is ed25519 private, public key

echo $1
echo '{
	"jsonrpc":"2.0",
	"id":1,
	"method":"author_insertKey",
	"params": ["babe", '\"$2\"', '\"$3\"']
}'
echo '{
	"jsonrpc":"2.0",
	"id":1,
	"method":"author_insertKey",
	"params": ["gran", '\"$4\"', '\"$5\"']
}'
echo '{
	"jsonrpc":"2.0",
	"id":1,
	"method":"author_insertKey",
	"params": ["imon", '\"$2\"', '\"$3\"']
}'
echo $'{
	"jsonrpc":"2.0",
	"id":1,
	"method":"author_insertKey",
	"params": ["audi", '\"$2\"', '\"$3\"']
}'

curl $1 -H "Content-Type:application/json;charset=utf-8" -d \
	'{
		"jsonrpc":"2.0",
		"id":1,
		"method":"author_insertKey",
		"params": ["babe", '\"$2\"', '\"$3\"']
	}'
curl $1 -H "Content-Type:application/json;charset=utf-8" -d \
	'{
		"jsonrpc":"2.0",
		"id":1,
		"method":"author_insertKey",
		"params": ["gran", '\"$4\"', '\"$5\"']
	}'
curl $1 -H "Content-Type:application/json;charset=utf-8" -d \
	'{
		"jsonrpc":"2.0",
		"id":1,
		"method":"author_insertKey",
		"params": ["imon", '\"$2\"', '\"$3\"']
	}'
curl $1 -H "Content-Type:application/json;charset=utf-8" -d \
	'{
		"jsonrpc":"2.0",
		"id":1,
		"method":"author_insertKey",
		"params": ["audi", '\"$2\"', '\"$3\"']
	}'
