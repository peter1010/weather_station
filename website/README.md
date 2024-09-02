# Web server part of weather station project

Based on Lighttpd

Uses mod_scgi to communicate with bespoke app

App gets weather data from postgresql server

# Creating a self-signed certificate for testing

-- Create Root CA private Key --
$ openssl genpkey -algorithm RSA -des3 -pkeyopt rsa_keygen_bits:2048 -out example_ca.key


-- Create some Root CA info in a suitable config file --
See example_ca.conf


-- Create Root CA certificate from the private key in previous step --
$ openssl req -x509 -new -key my_ca.key -sha256 -days 1000 -config example_ca.conf -out example_ca.pem


-- Create Another private key --
$ openssl genpkey -algorithm RSA -des3 -pkeyopt rsa_keygen_bits:2048 -out example_website.key
Remove -des3 if password protection is not required

-- Create some Website info in a suitable config file --
See example_website.conf


-- Create a CSR ...
$ openssl req -new -key example_website.key -sha256 -config example_website.conf -out example_website.csr


-- Create extend info for certificate ---
See example_website_ext

-- Sign it --

openssl x509 -req -in example_website.csr -CA example_ca.pem -CAkey example_ca.key \
-CAcreateserial -out example_website.crt -days 825 -sha256 -extfile example_website.ext


# scgi_weather_app

Install the python packages "pip", "setuptools" and "build".

To build the package...

python -m build

To install

pip install  ....




