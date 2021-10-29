import sys
import numpy as np
import matplotlib
from matplotlib import pyplot as plt

ts = []

file = open(f"log.txt","r")
logs=file.readlines()
file.close()

for line in logs:
	split_line = line.strip().split()
	if len(split_line) == 1:
		ts.append(round(int(split_line[0])/1000))
start = ts[0]
for i in range(len(ts)):
	ts[i] = ts[i] - start

timeline = range(600)
count = []
expected = []
for n in timeline:
	count.append(len([i for i in ts if i < 1000*n]))
	expected.append(500)

fig, ax = plt.subplots()
ax.plot(timeline,count,'-b')
ax.plot(timeline,expected,'--r',label='epoch size')
ax.set_title("Chain growth in Bitcoin")
ax.set_ylabel('num of blocks')
ax.set_xlabel('time(s)')
ax.legend(loc="lower right")
plt.show()



