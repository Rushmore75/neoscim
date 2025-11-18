datafile = '$FILE'
set datafile separator ','

set title '$TITLE'
set key autotitle columnhead
set xlabel "$XLABEL"
set ylabel "$YLABEL"

set style line 1 linewidth 2 linecolor 1 pointtype 7 pointsize 1.5

set autoscale
set grid

set term png size 1280, 720
set output '$OUTPUT'
plot datafile using 1:2 with linespoints linestyle 1 
replot
