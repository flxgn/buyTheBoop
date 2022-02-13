import requests
import os
import base64
import hashlib
import json
import hmac
import time
from datetime import datetime


def make_request(after):
    request_path = '/api/v5/market/history-candles?instId=BTC-USDT&after=' + str(after)
    now = datetime.utcnow().isoformat()[:-3]+'Z'
    msg = '{0}{1}{2}'.format(now, 'GET', request_path)
    signature = hmac.new(
        os.getenv('OK_ACCESS_SECRET').encode('utf-8'),
        msg=msg.encode('utf-8'),
        digestmod=hashlib.sha256
    ).digest()
    headers = {
        'accept': 'application/json',
        'content-type': "application/json",
        'OK-ACCESS-KEY': os.getenv('OK_ACCESS_KEY'),
        'OK-ACCESS-SIGN': base64.b64encode(signature).decode(),
        'OK-ACCESS-TIMESTAMP': now,
        'OK-ACCESS-PASSPHRASE': os.getenv('OK_ACCESS_PASSPHRASE')
    } 
    return requests.get('https://www.okx.com' + request_path, headers=headers)

def fetch():
    after = int(datetime.utcnow().timestamp()) * 1000
    result = []
    for _ in range(1000):
        
        resp = make_request(after)
        
        if resp.status_code != 200:
            print(resp.status_code)
            print(resp.json())
        for entry_tuple in resp.json()['data']:
            entry = dict(
                time=entry_tuple[0],
                open=entry_tuple[1],
                high=entry_tuple[2],
                low=entry_tuple[3],
                close=entry_tuple[4],
                volume_base=entry_tuple[5],
                volume_quote=entry_tuple[6]
            )
            result.append(entry)
            after = entry_tuple[0]
    with open('data.json', 'w') as f:
        json.dump(result, f)


if __name__ == '__main__':
    fetch()
