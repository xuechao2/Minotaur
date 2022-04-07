import sys
import numpy as np
import matplotlib
from matplotlib import pyplot as plt

ts = []

file = open(f"log_B.txt","r")
logs=file.readlines()
file.close()

for line in logs:
	split_line = line.strip().split()
	if len(split_line) == 1:
		ts.append(round(int(split_line[0])/1000))
start = ts[0]
for i in range(len(ts)):
	ts[i] = ts[i] - start

ts_m = []

file_m = open(f"log_M.txt","r")
logs=file_m.readlines()
file_m.close()

for line in logs:
	split_line = line.strip().split()
	if len(split_line) == 1:
		ts_m.append(round(int(split_line[0])/1000))
start = ts_m[0]
for i in range(len(ts_m)):
	ts_m[i] = ts_m[i] - start

timeline = range(600)
count = []
count_m = []
expected = []
for n in timeline:
	count.append(len([i for i in ts if i < 1000*n]))
	count_m.append(4*len([i for i in ts_m if i < 1000*n]))
	expected.append(400)

ts_f = []

file_f = open(f"log_F.txt","r")
logs=file_f.readlines()
file_f.close()

for line in logs:
	split_line = line.strip().split()
	if len(split_line) == 1:
		ts_f.append(round(int(split_line[0])/1000))
start = ts_f[0]
for i in range(len(ts_f)):
	ts_f[i] = ts_f[i] - start

timeline = range(600)
count = []
count_m = []
count_f = []
expected = []
for n in timeline:
	count.append(len([i for i in ts if i < 1000*n]))
	count_m.append(4*len([i for i in ts_m if i < 1000*n]))
	count_f.append(len([i for i in ts_f if i < 1000*n]))
	expected.append(400)



fig, ax = plt.subplots()
ax.plot(timeline,count,'-b',label='Bitcoin')
ax.plot(timeline,count_f,'-y',label='FruitChain')
ax.plot(timeline,count_m,'-g',label='Minotaur')
ax.plot(timeline,expected,'--r',label='epoch size')
ax.set_title("Growth of PoW blocks")
ax.set_ylabel('num of blocks')
ax.set_xlabel('time(s)')
ax.legend(loc="upper left")
plt.show()



