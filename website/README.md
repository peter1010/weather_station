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

# Creating a self-signed certificate for testing

-- Create Root CA private Key --
$ openssl genpkey -algorithm RSA -des3 -pkeyopt rsa_keygen_bits:2048 -out my_ca.key

-- Create some Root CA info in a suitable config file --

[req]
distinguished_name = req_distinguished_name
prompt = no
[req_distinguished_name]
C = UK
ST = Wales
L = Newtown
O = MyCompany
OU = MyDivision
CN = MyCommonName


-- Create Root CA certificate from the private key in previous step --
$ openssl req -x509 -new -key my_CA.key -sha256 -days 1000 -config ca_req.cnf -out my_CA.pem


-- Create Another private key --
$ openssl genpkey -algorithm RSA -des3 -pkeyopt rsa_keygen_bits:2048 -out my_website.key


-- Create some Website info in a suitable config file --

[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no
[req_distinguished_name]
C = UK
ST = Wales
L = Newtown
O = MyCompany
OU = MyDivision
CN = www.company.com


-- Create a CSR ...
$ openssl req -new -key my_website.key -sha256 -config my_website.conf -out my_website.csr

-- Create extend info for certificate ---


authorityKeyIdentifier = keyid,issuer
basicConstraints = CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names
[alt_names]
DNS.1 = www.company.com
DNS.2 = company.com
DNS.3 = company.net


-- Sign it --

openssl x509 -req -in website.csr -CA my_ca.pem -CAkey my_ca.key \
-CAcreateserial -out webiste.crt -days 825 -sha256 -extfile website.ext





