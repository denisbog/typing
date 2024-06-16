### run
```bash
trunk serve --open
```
### update styles
```bash
npx tailwindcss -i ./input.css -o ./output.css --watch
```
### format letptos code
```bash
leptosfmt .
```
### run translation server

using https://huggingface.co/docs/transformers/model_doc/marian

#### dependencies

AWS EC2

use 16GB of storage (after the installation uses 7.8GB of store, during the installation up to 12GB)

use t2.medium to install and t2.small to run

```python
sudo yum install python pip
mkdir tmp
TMPDIR=/home/ec2-user/tmp pip install torch flask transformers sentencepiece sacremoses --no-cache-dir
```

```python
python translate_server.py
```
