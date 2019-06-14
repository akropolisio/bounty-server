# Setup

Check out [setup.md](setup.md).


# Usage

```
/1.0/ GET & POST
/1.0/get?address=0xFOO&recaptcha=RECAPTCHA

curl -S --header "Content-Type: application/json" --request GET --data '{"address":"0xBOO", "recaptcha":"recaptcha"}'  http://127.0.0.1:8080/1.0/

curl -S --header "Content-Type: application/json" --request POST \
--data '{"not_resident":true,"terms":true,"address":"0xBOO","recaptcha":"test-value"}' \
http://127.0.0.1:8080/1.0/
```
