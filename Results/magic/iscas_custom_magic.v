// NOR_NOT mapped module module_name

module module_name (
  input  ip_1,
  input  ip_2,
  output op_1,
  output op_2
);

  wire wr_3;
  wire wr_4;
  wire wr_5;
  wire wr_6;
  wire wr_7;

  not    g1   ( wr_3     ,           ip_2     );
  not    g2   ( wr_5     ,           ip_1     );
  nor    g3   ( wr_4     , ip_1     , wr_3     );
  nor    g4   ( wr_6     , ip_2     , wr_5     );
  nor    g5   ( wr_2     , wr_3     , wr_5     );
  nor    g6   ( wr_7     , wr_6     , wr_4     );
  not    g7   ( wr_1     ,           wr_7     );

endmodule
