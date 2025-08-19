A GUI app written in RUST to study / demo how UDP and TCP works  
you can use this app when you want to listen to or send some traffic quickly to network

![main_gui](assets/demo1.png)

# Use the APP
build and run the app by double clicking  
the scripts are used to start / stop multiple instances
so that we can run tests on a single computer

this by default starts 4 instances
```bash
./start_many.sh
./stop_many.sh
```

or use parameters
```bash
./start_many.sh [count]
./stop_many.sh
```
`count` should be >= 1  
in theory you can start as many instances as you wish (not tested)


by default the script builds the app in debug mode
to build in release mode use `--release` flag
```bash
./start_many.sh --release
./stop_many.sh
```

# Some notes
the UDP broadcast feature is not fully tested
