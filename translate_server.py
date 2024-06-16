from transformers import MarianMTModel, MarianTokenizer
model_name = "Helsinki-NLP/opus-mt-de-en"
tokenizer = MarianTokenizer.from_pretrained(model_name)
model = MarianMTModel.from_pretrained(model_name)

import json
from flask import Flask, request, jsonify

app = Flask(__name__)
@app.route('/translate', methods=['POST'])
def translate():
    src_text = json.loads(request.data)["src"]
    # src_text = request.args.get('text')
    # src_text = ["was machen wir jetzt?"]
    translated = model.generate(**tokenizer(src_text, return_tensors="pt", padding=True))
    return jsonify({'translated': [tokenizer.decode(t, skip_special_tokens=True) for t in translated]})
print ("start application")
app.run(debug=False)
