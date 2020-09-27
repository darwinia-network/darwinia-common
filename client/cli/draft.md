
# Darwinia Node Template CLI

## Specifications
Specifications of the commands in the template can be found in [Substrate Developer Hub]('https://substrate.dev/docs/en/knowledgebase/getting-started/#manual-installation) 

## Running Darwinia Node Template
Running a node at darwinia using node template in easy steps and development tokens are given by defualt.

 ***[See at Darwinia for basic setup](https://github.com/darwinia-network/darwinia#building)***


**Download Node-Template**
```
$ git clone https://github.com/darwinia-network/darwinia-common
$ cd darwinia-common
```

**For Building**
```
$ cargo build --release
``` 

**For Running**
```
$ cd target/release/
$ ./node-template --dev 
```

## Darwinia App UI 
In order to display current metics for our node, use the [apps.darwinia.network](https://apps.darwinia.network). And before running the node please use these commands.

`$ ./node-template --dev --ws-external --ws-port 9944`

Now our node is up and running, and also generating blocks. 
At the app ui, click the (Icon image) to change desired network.

![showapps](https://res.cloudinary.com/dunig2dhs/image/upload/v1601188421/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03_1.jpg)

At the list, it shows the listed networks available to use. Right now we are using the localhost. So click the **Local Node**, and after click <u>save and reload</u>

![showlistednetworks](https://res.cloudinary.com/dunig2dhs/image/upload/v1601189339/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03_2.jpg)

after you have save and reload the app, you will see basic account for alice and other users for development. 

![showdisplaysuccess](https://res.cloudinary.com/dunig2dhs/image/upload/v1601187880/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03.jpg)

### Congratulations you have sucessfully run your own development node for darwinia.





 