
compile:
	rm -rf target
	solcjs BRC20_Controller.sol --bin --abi
	find . -type f -name "*.abi" -exec sh -c 'python3 -m json.tool "$$1" > "$$1.tmp" && mv "$$1.tmp" "$$1"' _ {} \;
	mkdir -p target
	mv *.abi target/
	mv *.bin target/
