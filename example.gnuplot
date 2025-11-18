datafile = 'data.csv'
set datafile separator ','

set title 'Probably the filename'
set key autotitle columnhead
set xlabel "x"
set ylabel "y"

set style line 1 linewidth 2 linecolor 1 pointtype 7 pointsize 1.5

set autoscale
set grid

set term png size 1280, 720
set output 'output.png'
plot datafile using 1:2 with linespoints linestyle 1 
replot
