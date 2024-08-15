# Web server part of weather station project


Based on Lighttpd

Uses mod_scgi to communicate with bespoke python app

App gets weather data from postgresql server

# scgi_weather_app

Install the python packages "pip", "setuptools" and "build".

To build the package...

python -m build

To install

pip install  ....
