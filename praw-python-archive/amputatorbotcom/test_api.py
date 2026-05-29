import unittest

import requests

from static import static


class MyTestCase(unittest.TestCase):
    def test_api(self):
        # amount_of_urls =
        access_token = next(iter(static.API_KEYS))
        base_api_url = "https://www.amputatorbot.com/api/v1/convert?"
        api_url_append1 = "q=https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/"
        url = base_api_url + api_url_append1
        result = requests.get(url, headers={f'Content-Type': 'application/json',
                                            'Authorization': 'Bearer ' + access_token})

        print(result.text)
        self.assertEqual(True, True)


if __name__ == '__main__':
    unittest.main()
