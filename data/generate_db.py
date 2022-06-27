import os
import base64
import json

def get_random(length):
  buf = os.urandom(length)
  return base64.b64encode(buf).decode("utf-8")

def get_all_ones(length):
  buf = b"\xFF"*length
  return base64.b64encode(buf).decode("utf-8")

if __name__ == "__main__":
  all_ones = os.getenv("DB_ALL_ONES")
  number_of_entries_exp = os.getenv("DB_NUM_ENTRIES_EXP") or 16
  element_size = os.getenv("DB_ELEMENT_SIZE_EXP") or 13
  v = int(number_of_entries_exp)
  num_entries = pow(2, v)
  out = []
  if all_ones == "0":
    for i in range(num_entries):
      out.append(get_random(pow(2, int(element_size))))
  else:
    for i in range(num_entries):
      out.append(get_all_ones(pow(2, int(element_size))))
  file_name = os.getenv("DB_OUTPUT_PATH")
  if not file_name:
    file_name = "rand_db.json"
  print("Printing {} entries to {}".format(num_entries, file_name))
  with open(file_name, 'w', encoding='utf-8') as f:
    json.dump(out, f, ensure_ascii=False)