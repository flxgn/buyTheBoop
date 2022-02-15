import json
from datetime import datetime

def chunks(lst, n):
    """Yield successive n-sized chunks from lst."""
    for i in range(0, len(lst), n):
        yield lst[i:i + n]

def run():
    data = []
    with open('data.json', 'r') as f:
        data = json.load(f)

    result = []
    chunk_size = 5
    for entry_chunk in chunks(data, chunk_size):
        summation = 0
        last_entry = {}
        for entry in entry_chunk:
            summation += entry['close']
            last_entry = entry
        last_entry['close'] = summation / chunk_size
        result.append(last_entry)

    with open('data_5min.json', 'w') as f:
        json.dump(result, f)


if __name__ == '__main__':
    run()
