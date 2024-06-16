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

```python
python translate_server.py
```
