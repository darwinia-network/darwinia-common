
# Darwinia CLI Interface

## Specifications
Specifications of the commands in the template can be found in [Substrate Developer Hub]('https://substrate.dev/docs/en/knowledgebase/getting-started/#manual-installation) 

## Running Darwinia Node Template
Running a node at darwinia using node template in easy steps and development tokens are given by default.

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
To display current metics for our node, use the [apps.darwinia.network](https://apps.darwinia.network). And before running the node please use the following commands.

`$ ./node-template --dev --ws-external --ws-port 9944`

Now our node is up and running, and also generating blocks. 
At the app ui, click the (Icon image) to change the desired network.

![showapps](https://res.cloudinary.com/dunig2dhs/image/upload/v1601188421/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03_1.jpg)

The list, it shows the listed networks available to use. Right now we are using the localhost. So click the **Local Node**, and after click <u>save and reload</u>

![showlistednetworks](https://res.cloudinary.com/dunig2dhs/image/upload/v1601189339/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03_2.jpg)

after you have saved and reload the app, you will see the basic account for Alice and other users for development.

![showdisplaysuccess](https://res.cloudinary.com/dunig2dhs/image/upload/v1601187880/darwinia/documentation/Screenshot_from_2020-09-27_14-22-03.jpg)

### Congratulations you have successfully run your development node for Darwinia.

## Shortcuts

The node has different shortcuts aside alice and bob. And these are the following:

**--bob**           

**--alice**    

**--charlie**

**--eve**       

**--ferdie**

**--dave**

**--one**

**--two**

Example:
```
input: --alice
output: --validator --name alice 
```
session keys for each shortcuts aside for *one and two* are stored at keystore. 

## Sentry 

Deploying Sentry Nodes will be depreciated by October 2020. See at [substrate issues](https://github.com/paritytech/substrate/issues/6845) for more information regarding the depreciation of Sentry Nodes.





 