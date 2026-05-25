class Page:
    def __init__(self, current_url=None, status_code=None, title=None, soup=None):
        self.current_url = current_url
        self.status_code = status_code
        self.title = title
        self.soup = soup
